use super::BlockSectionRouter;
#[cfg(any(feature = "tempdb", feature = "dynamodb"))]
use crate::database::{create_response, delete_response, update_response};
use crate::{
    manager_ui::MetaForManagerView, ManagerViewModes, MessageAction, ReceptionistAction,
    ReceptionistResponse, SlackResponseAction, ViewBlockStateType,
};

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{from_str, from_value};
use slack_morphism::prelude::*;
use std::{collections::HashMap, str::FromStr};
use tracing::info;

pub async fn process_submission_event(
    submission_event: SlackInteractionViewSubmissionEvent,
) -> Result<Option<SlackResponseAction>> {
    let user_id: String = submission_event.user.id.to_string();

    match submission_event.view.view {
        SlackView::Home(_home_view) => {
            bail!("Home views are unimplemented");
        }
        SlackView::Modal(modal) => {
            let private_metadata: MetaForManagerView = from_str(
                &modal
                    .private_metadata
                    .ok_or_else(|| anyhow!("metadata not found"))?,
            )?;

            let view_state_params = submission_event.view.state_params;

            let state = view_state_params.state.ok_or_else(|| {
                anyhow!(
                    "no state in view submission params. User: {:?}",
                    submission_event.user
                )
            })?;

            let block_id_map = extract_action_block_states(state)?;
            let parsed_view =
                parse_manager_block_states(&block_id_map, private_metadata, &user_id)?;

            return match parsed_view.mode {
                ManagerViewModes::Home => Ok(None),
                ManagerViewModes::CreateResponse => {
                    // get response info from view states
                    match parsed_view.response.validate() {
                        Some(validation_errors) => Ok(Some(
                            SlackResponseAction::from_validation_errors(validation_errors),
                        )),
                        None => {
                            create_response(parsed_view.response).await?;
                            Ok(None)
                        }
                    }
                }
                ManagerViewModes::EditResponse => {
                    // get response_id from selector
                    match parsed_view.response.validate() {
                        Some(validation_errors) => Ok(Some(
                            SlackResponseAction::from_validation_errors(validation_errors),
                        )),
                        None => {
                            update_response(parsed_view.response).await?;
                            Ok(None)
                        }
                    }
                }
                ManagerViewModes::DeleteResponse => {
                    // get_response_id from selector
                    // delete_response(user_id, selected_response_id)
                    delete_response(parsed_view.response).await?;
                    Ok(None)
                }
            };
        }
    }
}

/// HashMap<ActionBlockId, ViewBlockStateType>
fn extract_action_block_states(
    view_state: SlackViewState,
) -> Result<HashMap<String, ViewBlockStateType>> {
    let mut block_state_map: HashMap<String, ViewBlockStateType> = HashMap::new();

    for (_block_id, action_block_map) in view_state.values {
        for (action_id, block_state_value) in action_block_map
            .as_object()
            .ok_or_else(|| anyhow!("view submission values are not a Map Object"))?
        {
            let as_state_value: ViewBlockStateType = from_value(block_state_value.to_owned())
                .with_context(|| {
                    format!(
                        "Add a new variant to ViewBlockStateType enum that fits this structure: {block_state_value}"
                    )
                })?;

            block_state_map.insert(action_id.to_owned(), as_state_value);
        }
    }

    Ok(block_state_map)
}

#[derive(Debug)]
pub struct ParsedManagerViewSubmission {
    pub mode: ManagerViewModes,
    pub selected_response_id: Option<String>,
    pub response: ReceptionistResponse,
}

fn parse_manager_block_states(
    block_id_map: &HashMap<String, ViewBlockStateType>,
    private_metadata: MetaForManagerView,
    user_id: &str,
) -> Result<ParsedManagerViewSubmission> {
    let mut parsed_submission = ParsedManagerViewSubmission {
        mode: private_metadata.current_mode.to_owned(),
        response: private_metadata.response.unwrap_or_default(),
        selected_response_id: None,
    };

    // exit early if home view, no saving to database
    if matches!(parsed_submission.mode, ManagerViewModes::Home) {
        info!("Home View submitted");
        return Ok(parsed_submission);
    }

    for (action_id_str, block_state) in block_id_map {
        let (route, index_result) = BlockSectionRouter::from_string_with_index(action_id_str)
            .ok_or_else(|| anyhow!("route not found"))?;

        match route {
            BlockSectionRouter::ManagerModeSelection => {
                parsed_submission.mode =
                    ManagerViewModes::from_str(&block_state.get_value_from_static_select()?)?
            }
            BlockSectionRouter::ActionTypeSelected => parsed_submission
                .response
                .update_action_type(&block_state.get_value_from_static_select()?, index_result?)?,
            BlockSectionRouter::ConditionTypeSelected => {
                parsed_submission.response.update_condition_type(
                    &block_state.get_value_from_static_select()?,
                    index_result?,
                )?
            }
            BlockSectionRouter::ListenerChannelSelected => {
                if let Some(channel_id) = block_state.get_conversation_select_value()? {
                    parsed_submission
                        .response
                        .update_slack_channel(channel_id.to_string())?;
                }
            }
            BlockSectionRouter::MessageConditionValueInput => {
                parsed_submission.response.update_message_condition_string(
                    block_state.get_plain_text_value()?,
                    index_result?,
                )?;
            }
            BlockSectionRouter::AttachEmojiInput => {
                let action = parsed_submission.response.get_action_mut(index_result?)?;

                match action {
                    ReceptionistAction::ForMessage(msg_action) => {
                        *msg_action = match msg_action {
                            MessageAction::AttachEmoji(_) => {
                                MessageAction::AttachEmoji(block_state.get_plain_text_value()?)
                            }
                            _ => bail!("wrong action type for emoji input: {}", msg_action),
                        };
                    }
                }
            }

            BlockSectionRouter::ResponseSelection => {
                parsed_submission.response.id = block_state.get_value_from_static_select()?
            }

            BlockSectionRouter::CollaboratorSelection => {
                parsed_submission.response.collaborators = block_state
                    .get_multi_users_select_value()?
                    .iter()
                    .map(|u| u.to_owned())
                    .collect();
            }
            BlockSectionRouter::ReplyThreadedMsgInput => {
                let action = parsed_submission.response.get_action_mut(index_result?)?;

                match action {
                    ReceptionistAction::ForMessage(msg_action) => {
                        *msg_action = match msg_action {
                            MessageAction::ThreadedMessage(_) => {
                                MessageAction::ThreadedMessage(block_state.get_plain_text_value()?)
                            }
                            _ => bail!("wrong action type for emoji input"),
                        };
                    }
                }
            }
            BlockSectionRouter::PostChannelMsgInput => {
                let action = parsed_submission.response.get_action_mut(index_result?)?;

                match action {
                    ReceptionistAction::ForMessage(msg_action) => {
                        *msg_action = match msg_action {
                            MessageAction::ChannelMessage(_) => {
                                MessageAction::ChannelMessage(block_state.get_plain_text_value()?)
                            }
                            _ => bail!("wrong action type for emoji input"),
                        };
                    }
                }
            }
            BlockSectionRouter::PDEscalationPolicyInput => {
                let action = parsed_submission.response.get_action_mut(index_result?)?;

                match action {
                    ReceptionistAction::ForMessage(msg_action) => {
                        *msg_action = match msg_action {
                            MessageAction::MsgOncallInThread { message, .. } => {
                                MessageAction::MsgOncallInThread {
                                    escalation_policy_id: block_state.get_plain_text_value()?,
                                    message: std::mem::take(message),
                                }
                            }
                            _ => bail!("wrong action type for emoji input"),
                        };
                    }
                }
            }
            BlockSectionRouter::PDThreadedMsgInput => {
                let action = parsed_submission.response.get_action_mut(index_result?)?;

                match action {
                    ReceptionistAction::ForMessage(msg_action) => {
                        *msg_action = match msg_action {
                            MessageAction::MsgOncallInThread {
                                escalation_policy_id,
                                ..
                            } => MessageAction::MsgOncallInThread {
                                escalation_policy_id: std::mem::take(escalation_policy_id),
                                message: block_state.get_plain_text_value()?,
                            },
                            _ => bail!("wrong action type for emoji input"),
                        };
                    }
                }
            }
            BlockSectionRouter::FwdMsgToChanChannelInput => {
                let action = parsed_submission.response.get_action_mut(index_result?)?;

                match action {
                    ReceptionistAction::ForMessage(msg_action) => {
                        *msg_action = match msg_action {
                            MessageAction::ForwardMessageToChannel { msg_context, .. } => {
                                if let Some(channel_id) =
                                    block_state.get_conversation_select_value()?
                                {
                                    MessageAction::ForwardMessageToChannel {
                                        channel: channel_id.to_string(),
                                        msg_context: std::mem::take(msg_context),
                                    }
                                } else {
                                    bail!("No channel_id provided in channel update event")
                                }
                            }
                            _ => bail!("wrong action type for Forward Message - Channel Input"),
                        };
                    }
                }
            }
            BlockSectionRouter::FwdMsgToChanMsgContextInput => todo!(),
        }
    }

    if !parsed_submission
        .response
        .collaborators
        .contains(&user_id.into())
    {
        parsed_submission
            .response
            .collaborators
            .push(user_id.to_owned())
    }

    Ok(parsed_submission)
}
