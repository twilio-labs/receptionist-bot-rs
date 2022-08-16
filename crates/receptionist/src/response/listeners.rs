use crate::{BlockSectionRouter, EnumUtils, SlackBlockValidationError};
use serde::{Deserialize, Serialize};
use slack_morphism::prelude::*;
use strum::{EnumIter, EnumString};

#[derive(EnumUtils!, Serialize, Deserialize, EnumString, Clone)]
#[serde(tag = "listener_type", rename_all = "snake_case")]
#[strum(serialize_all = "kebab_case")]
pub enum ListenerEvent {
    SlackChannelMessage { channel_id: String },
    // SlackSlashCommand,
}

impl Default for ListenerEvent {
    fn default() -> Self {
        Self::SlackChannelMessage {
            channel_id: "".into(),
        }
    }
}

impl ListenerEvent {
    pub fn matches_slack_channel_id(&self, incoming_channel: &str) -> bool {
        match self {
            ListenerEvent::SlackChannelMessage { channel_id } => channel_id == incoming_channel,
        }
    }

    pub fn validate(&self) -> Option<SlackBlockValidationError> {
        match self {
            ListenerEvent::SlackChannelMessage { channel_id } => {
                if channel_id.is_empty() {
                    Some(SlackBlockValidationError {
                        block_id: BlockSectionRouter::ListenerChannelSelected.to_block_id(None),
                        error_message: "No channel selected".to_string(),
                    })
                } else {
                    None
                }
            }
        }
    }

    pub fn default_blocks() -> Vec<SlackBlock> {
        slack_blocks![
            some_into(
                SlackSectionBlock::new()
                    .with_text(md!(":slack: Select a Channel"))
                    .with_accessory(SlackSectionBlockElement::ConversationsSelect(
                        SlackBlockConversationsSelectElement::new(
                            BlockSectionRouter::ListenerChannelSelected.to_action_id(None),
                            pt!("#my-channel"),
                        ),
                    ))
                    .with_block_id(BlockSectionRouter::ListenerChannelSelected.to_block_id(None))
            ),
            some_into(SlackDividerBlock::new())
        ]
    }

    pub fn to_editor_blocks(&self) -> Vec<SlackBlock> {
        match self {
            ListenerEvent::SlackChannelMessage { channel_id } => {
                let conversations_select_element = SlackBlockConversationsSelectElement::new(
                    BlockSectionRouter::ListenerChannelSelected.to_action_id(None),
                    pt!("#my-channel"),
                );

                let conversations_select_element = if !channel_id.is_empty() {
                    conversations_select_element.with_initial_conversation(channel_id.into())
                } else {
                    conversations_select_element
                };

                slack_blocks![
                    some_into(
                        SlackSectionBlock::new()
                            .with_text(md!(
                                ":slack: Select a Channel                   :point_right:"
                            ))
                            .with_accessory(SlackSectionBlockElement::ConversationsSelect(
                                conversations_select_element
                            ))
                            .with_block_id(
                                BlockSectionRouter::ListenerChannelSelected.to_block_id(None)
                            )
                    ),
                    some_into(SlackDividerBlock::new())
                ]
            }
        }
    }
}
