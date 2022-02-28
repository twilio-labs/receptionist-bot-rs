use super::BlockSectionRouter;
#[cfg(any(feature = "tempdb", feature = "dynamodb"))]
use crate::database::get_response_by_id;
use crate::{
    manager_ui::{select_mode, MetaForManagerView},
    SlackStateWorkaround,
};
use anyhow::{anyhow, bail, Context, Result};
use slack_morphism::prelude::*;

pub async fn process_action_event(
    actions_event: SlackInteractionBlockActionsEvent,
    slack: &SlackStateWorkaround,
) -> Result<()> {
    let slack_view = &actions_event
        .view
        .ok_or_else(|| anyhow!("Not a view, message block events are unimplemented"))?;

    match slack_view {
        SlackView::Home(_) => bail!("Home view actions are unimplemented"),
        SlackView::Modal(modal) => {
            let view_id = match actions_event.container {
                SlackInteractionActionContainer::View(container) => container.view_id,
                _ => bail!("invalid state, message events not supported from within a modal"),
            };

            let metadata_str = modal
                .private_metadata
                .as_deref()
                .ok_or_else(|| anyhow!("no private_metadata field in view"))?;

            let mut private_metadata: MetaForManagerView = serde_json::from_str(metadata_str)
                .with_context(|| format!("invalid private metadata: {metadata_str}"))?;

            for action in actions_event
                .actions
                .ok_or_else(|| anyhow!("No actions in action event"))?
            {
                let (route, index_result) =
                    BlockSectionRouter::from_action_id_with_index(&action.action_id)
                        .ok_or_else(|| anyhow!("block action did not match any router ids"))?;

                // Process any Block actions as the input changes (before final submission)
                // Handling this isn't necessary for most event types, just when the form needs to be updated
                //   Example: when a Condition Type or Action Type changes and you need to render new fields
                match route {
                    BlockSectionRouter::ManagerModeSelection => {
                        let selected_item = action.selected_option.ok_or_else(|| {
                            anyhow!("No selected item for manager mode selection")
                        })?;

                        let new_metadata = select_mode(&selected_item.value, &private_metadata)?;

                        slack
                            .update_manager_modal_view(view_id.to_owned(), &new_metadata)
                            .await?
                    }
                    BlockSectionRouter::ResponseSelection => {
                        let selected_item = action
                            .selected_option
                            .ok_or_else(|| anyhow!("No selection for response"))?;

                        private_metadata.response =
                            Some(get_response_by_id(&selected_item.value).await?);

                        slack
                            .update_manager_modal_view(view_id.to_owned(), &private_metadata)
                            .await?
                    }
                    BlockSectionRouter::ConditionTypeSelected => {
                        let mut response = private_metadata
                            .response
                            .ok_or_else(|| anyhow!("No Response in view metadata"))?;

                        let action_value = action
                            .selected_option
                            .ok_or_else(|| anyhow!("no option selected"))?
                            .value;

                        response.update_condition_type(&action_value, index_result?)?;

                        private_metadata.response = Some(response);

                        slack
                            .update_manager_modal_view(view_id.to_owned(), &private_metadata)
                            .await?
                    }
                    BlockSectionRouter::ActionTypeSelected => {
                        let mut response = private_metadata
                            .response
                            .ok_or_else(|| anyhow!("No Response in view metadata"))?;

                        let rec_action = response
                            .actions
                            .get_mut(index_result?)
                            .ok_or_else(|| anyhow!("action not found"))?;

                        rec_action.update_action_type_from_action_info(action)?;
                        private_metadata.response = Some(response);

                        slack
                            .update_manager_modal_view(view_id.to_owned(), &private_metadata)
                            .await?
                    }
                    BlockSectionRouter::CollaboratorSelection => {
                        todo!()
                    }
                    BlockSectionRouter::ListenerChannelSelected => {
                        todo!()
                    }
                    BlockSectionRouter::MessageConditionValueInput => {
                        todo!()
                    }
                    BlockSectionRouter::AttachEmojiInput => {
                        todo!()
                    }
                    BlockSectionRouter::ReplyThreadedMsgInput => {
                        todo!()
                    }
                    BlockSectionRouter::PostChannelMsgInput => {
                        todo!()
                    }
                    BlockSectionRouter::PDEscalationPolicyInput => todo!(),
                    BlockSectionRouter::PDThreadedMsgInput => todo!(),
                    BlockSectionRouter::FwdMsgToChanChannelInput => todo!(),
                    BlockSectionRouter::FwdMsgToChanMsgContextInput => todo!(),
                }
            }
            Ok(())
        }
    }
}
