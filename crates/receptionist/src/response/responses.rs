use crate::{
    add_emoji_colons, BlockSectionRouter, MessageAction, ReceptionistAction, ReceptionistCondition,
    ReceptionistListener, SlackBlockValidationError,
};
use anyhow::{anyhow, bail, Result};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use slack_morphism::prelude::*;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ReceptionistResponse {
    pub id: String,
    #[serde(flatten)]
    pub listener: ReceptionistListener,
    pub conditions: Vec<ReceptionistCondition>,
    pub actions: Vec<ReceptionistAction>,
    pub collaborators: Vec<String>,
}

impl Default for ReceptionistResponse {
    fn default() -> Self {
        let listener = ReceptionistListener::default();
        Self {
            id: Self::new_id(),
            actions: vec![ReceptionistAction::default_from_listener(&listener)],
            conditions: vec![ReceptionistCondition::default_from_listener(&listener)],
            collaborators: vec![],
            listener,
        }
    }
}

impl ReceptionistResponse {
    fn new_id() -> String {
        nanoid!()
    }

    pub fn new(
        collaborators: Vec<String>,
        listener: ReceptionistListener,
        actions: Vec<ReceptionistAction>,
        conditions: Vec<ReceptionistCondition>,
    ) -> Self {
        Self {
            id: Self::new_id(),
            listener,
            collaborators,
            actions,
            conditions,
        }
    }

    /// Check if any of this responses trigger conditions are met.
    /// conditions are not paired with a specific action, any trigger will fire all actions
    pub fn check_for_match(&self, message: &str) -> bool {
        for match_obj in &self.conditions {
            match &match_obj {
                ReceptionistCondition::ForMessage(msg_trigger) => {
                    if msg_trigger.should_trigger(message) {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn get_action_mut(&mut self, index: usize) -> Result<&mut ReceptionistAction> {
        self.actions
            .get_mut(index)
            .ok_or_else(|| anyhow!("action not found"))
    }

    pub fn to_editor_blocks(&self) -> Vec<SlackBlock> {
        let listener_blocks = self.listener.to_editor_blocks();

        let conditions_blocks: Vec<SlackBlock> = self
            .conditions
            .iter()
            .enumerate()
            .flat_map(|(index, condition)| condition.to_editor_blocks(Some(index)))
            .collect();

        let actions_blocks: Vec<SlackBlock> = self
            .actions
            .iter()
            .enumerate()
            .flat_map(|(index, action)| action.to_editor_blocks(Some(index)))
            .collect();

        // TODO collaborator blocks

        [
            self.build_collaborators_editor_blocks(),
            listener_blocks,
            conditions_blocks,
            actions_blocks,
        ]
        .concat()
    }

    pub fn update_condition_type(&mut self, type_str: &str, index: usize) -> Result<()> {
        let condition = self
            .conditions
            .get_mut(index)
            .ok_or_else(|| anyhow!("condition not found"))?;

        condition.update_condition_type(type_str)
    }

    pub fn update_action_type(&mut self, type_str: &str, index: usize) -> Result<()> {
        let action = self
            .actions
            .get_mut(index)
            .ok_or_else(|| anyhow!("condition not found"))?;

        action.update_action_type(type_str)
    }

    pub fn update_slack_channel(&mut self, conversation_id: String) -> Result<()> {
        match &self.listener {
            ReceptionistListener::SlackChannel { .. } => {
                self.listener = ReceptionistListener::SlackChannel {
                    channel_id: conversation_id,
                };
                Ok(())
            }
            #[allow(unreachable_patterns)]
            _ => bail!("Not a slack channel listener"),
        }
    }

    pub fn update_message_condition_string(&mut self, new_str: String, index: usize) -> Result<()> {
        let condition = self
            .conditions
            .get_mut(index)
            .ok_or_else(|| anyhow!("condition not found"))?;

        condition.update_message_condition_string(new_str)
    }

    pub fn validate(&self) -> Option<Vec<SlackBlockValidationError>> {
        let mut validation_errors: Vec<SlackBlockValidationError> = Vec::default();

        if let Some(validation_err) = self.listener.validate() {
            validation_errors.push(validation_err)
        }

        for (index, condition) in self.conditions.iter().enumerate() {
            if let Some(validation_err) = condition.validate(Some(index)) {
                validation_errors.push(validation_err)
            }
        }

        for (index, action) in self.actions.iter().enumerate() {
            if let Some(validation_err) = action.validate(Some(index)) {
                validation_errors.push(validation_err)
            }
        }

        // not necessary because collaborators will never be empty ?
        // if self.collaborators.is_empty() {
        //     validation_errors.push(SlackBlockValidationError {
        //         block_id: BlockSectionRouter::CollaboratorSelection.to_block_id(None),
        //         error_message: "empty".to_string(),
        //     })
        // }
        if !validation_errors.is_empty() {
            Some(validation_errors)
        } else {
            None
        }
    }

    fn build_collaborators_editor_blocks(&self) -> Vec<SlackBlock> {
        let multi_users_select_element = SlackBlockMultiUsersSelectElement::new(
            BlockSectionRouter::CollaboratorSelection.to_action_id(None),
            pt!("Select Collaborators"),
        );

        slack_blocks![
            some_into(
                SlackSectionBlock::new()
                    .with_text(md!(
                        ":busts_in_silhouette: Users that can edit this Response"
                    ))
                    .with_accessory(SlackSectionBlockElement::MultiUsersSelect(
                        multi_users_select_element
                    ))
                    .with_block_id(BlockSectionRouter::CollaboratorSelection.to_block_id(None))
            ),
            some_into(SlackDividerBlock::new())
        ]
    }

    /// Displays info about this entire Response on a single line in a "dropdown" selection box
    pub fn to_response_choice_item(&self) -> SlackBlockChoiceItem<SlackBlockPlainTextOnly> {
        let listener = match &self.listener {
            ReceptionistListener::SlackChannel { channel_id } => format!("#<#{channel_id}>"),
        };

        let actions: String = self
            .actions
            .iter()
            .map(|action| match action {
                ReceptionistAction::ForMessage(msg_act) => match msg_act {
                    MessageAction::AttachEmoji(emoji) => add_emoji_colons(emoji),
                    MessageAction::ThreadedMessage(msg) => msg.to_owned(),
                    MessageAction::ChannelMessage(msg) => msg.to_owned(),
                    MessageAction::MsgOncallInThread {
                        escalation_policy_id,
                        message,
                    } => format!(
                        "Tag Oncall: {escalation_policy_id} - {}..",
                        message.chars().take(10).collect::<String>()
                    ),
                    MessageAction::ForwardMessageToChannel {
                        channel,
                        msg_context,
                    } => format!(
                        "Fwd Message: {channel} - {}..",
                        msg_context.chars().take(10).collect::<String>()
                    ),
                },
            })
            .collect();

        let full_text = [listener, actions].join(" | ");

        SlackBlockChoiceItem::new(pt!(full_text), self.id.to_owned())
    }
}

pub fn mock_receptionist_response() -> ReceptionistResponse {
    ReceptionistResponse::new(
        vec!["some_slack_id".into()],
        ReceptionistListener::SlackChannel {
            channel_id: std::env::var("TEST_CHANNEL_ID")
                .unwrap_or_else(|_err| "<no_test_channel_set>".to_string()),
        },
        vec![ReceptionistAction::ForMessage(MessageAction::AttachEmoji(
            "thumbsup".to_string(),
        ))],
        vec![ReceptionistCondition::message_phrase("rust")],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, to_string_pretty};

    #[test]
    fn test_serde_enums() {
        let action_with_enum = mock_receptionist_response();

        let as_string = to_string_pretty(&action_with_enum);
        assert!(as_string.is_ok());
        let as_string = as_string.unwrap();
        print!("\n{}\n\n", &as_string);

        let deserialized = from_str::<ReceptionistResponse>(&as_string);
        assert!(deserialized.is_ok());
        let deserialized = deserialized.unwrap();

        assert_eq!(&action_with_enum, &deserialized);

        let back_to_string = to_string_pretty(&deserialized);
        assert!(back_to_string.is_ok());
        let back_to_string = back_to_string.unwrap();

        assert_eq!(back_to_string, as_string);
    }
}
