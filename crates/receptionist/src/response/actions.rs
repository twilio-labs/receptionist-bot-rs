use super::traits::{ForListenerEvent, SlackEditor};
use super::{listeners::ListenerEventDiscriminants, utils::slack_plain_text_input_block_for_view};
use crate::{BlockSectionRouter, EnumUtils, SlackBlockValidationError};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use slack_morphism::prelude::*;
use std::str::FromStr;
use strum::EnumString;

#[derive(EnumUtils!, Serialize, Deserialize, Clone, EnumString)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
#[strum(serialize_all = "kebab_case")]
pub enum Action {
    AttachEmoji(String),
    /// Post message in thread of the triggered message
    ThreadedMessage(String),
    /// Send message to same channel that triggered message
    ChannelMessage(String),
    MsgOncallInThread {
        escalation_policy_id: String,
        message: String,
    },
    /// Forward the triggered message to a different channel
    ForwardMessageToChannel {
        channel: String,
        msg_context: String,
    },
}

impl ForListenerEvent for Action {
    fn listeners(&self) -> Vec<ListenerEventDiscriminants> {
        match self {
            Action::AttachEmoji(_) => vec![ListenerEventDiscriminants::SlackChannelMessage],
            Action::ThreadedMessage(_) => vec![ListenerEventDiscriminants::SlackChannelMessage],
            Action::ChannelMessage(_) => vec![ListenerEventDiscriminants::SlackChannelMessage],
            Action::MsgOncallInThread { .. } => {
                vec![ListenerEventDiscriminants::SlackChannelMessage]
            }
            Action::ForwardMessageToChannel { .. } => {
                vec![ListenerEventDiscriminants::SlackChannelMessage]
            }
        }
    }

    fn default_from_listener(listener: &ListenerEventDiscriminants) -> Self {
        match listener {
            ListenerEventDiscriminants::SlackChannelMessage => Self::AttachEmoji(String::default()),
            // ListenerEventDiscriminants::SlackSlashCommand => Self::ForwardMessageToChannel {
            //     channel: String::default(),
            //     msg_context: String::default(),
            // },
        }
    }
}

impl SlackEditor for Action {
    fn to_description(&self) -> &str {
        match self {
            Action::AttachEmoji(_) => "Attach Emoji to Message",
            Action::ThreadedMessage(_) => "Reply with Threaded Message",
            Action::ChannelMessage(_) => "Post Message to Same Channel",
            Action::MsgOncallInThread { .. } => "Tag OnCall User in Thread",
            Action::ForwardMessageToChannel { .. } => {
                "Forward detected message to a different channel"
            }
        }
    }

    fn to_type_selector_blocks(&self, index: Option<usize>) -> Vec<SlackBlock> {
        slack_blocks![some_into(
            SlackSectionBlock::new()
                .with_text(md!(
                    ":building_construction: Select an Action to do if conditions are met"
                ))
                .with_accessory(SlackSectionBlockElement::StaticSelect(
                    SlackBlockStaticSelectElement::new(
                        BlockSectionRouter::ActionTypeSelected.to_action_id(index),
                        pt!("select action Type")
                    )
                    .with_options(Self::to_choice_items())
                    .with_initial_option(self.to_choice_item())
                ))
                .with_block_id(BlockSectionRouter::ActionTypeSelected.to_block_id(index))
        )]
    }

    fn to_value_input_blocks(&self, index: Option<usize>) -> Vec<SlackBlock> {
        match self {
            Action::AttachEmoji(emoji) => slack_plain_text_input_block_for_view(
                BlockSectionRouter::AttachEmojiInput,
                index,
                emoji.to_owned(),
                "my-emoji",
                "Choose an emoji (can also trigger Slack Workflows)",
            ),
            Action::ThreadedMessage(msg) => slack_plain_text_input_block_for_view(
                BlockSectionRouter::ReplyThreadedMsgInput,
                index,
                msg.to_owned(),
                "It looks like you're looking for..",
                "Enter a message to post in thread",
            ),
            Action::ChannelMessage(msg) => slack_plain_text_input_block_for_view(
                BlockSectionRouter::PostChannelMsgInput,
                index,
                msg.to_owned(),
                "Hey Channel..",
                "Enter Message to Post in Channel (not thread)",
            ),
            Action::MsgOncallInThread {
                escalation_policy_id,
                message,
            } => [
                slack_plain_text_input_block_for_view(
                    BlockSectionRouter::PDEscalationPolicyInput,
                    index,
                    escalation_policy_id.to_owned(),
                    "Pxxxxxx",
                    "Enter the escalation policy to query",
                ),
                slack_plain_text_input_block_for_view(
                    BlockSectionRouter::PDThreadedMsgInput,
                    index,
                    message.to_owned(),
                    "is oncall and will handle this.",
                    "Enter the message to provide in thread with the tagged user",
                ),
            ]
            .concat(),
            Action::ForwardMessageToChannel {
                channel,
                msg_context,
            } => {
                let mut channel_select = SlackBlockConversationsSelectElement::new(
                    BlockSectionRouter::FwdMsgToChanChannelInput.to_action_id(index),
                    pt!("#my-channel"),
                );

                if !channel.is_empty() {
                    channel_select = channel_select.with_initial_conversation(channel.into())
                }

                [
                    slack_blocks![
                        some_into(
                            SlackSectionBlock::new()
                                .with_text(md!(
                                    ":envelope_with_arrow: Select a Channel to forward this message to"
                                ))
                                .with_accessory(SlackSectionBlockElement::ConversationsSelect(
                                    channel_select
                                ))
                                .with_block_id(
                                    BlockSectionRouter::FwdMsgToChanChannelInput.to_block_id(index)
                                )
                        ),
                        some_into(SlackDividerBlock::new())
                    ],
                    slack_plain_text_input_block_for_view(
                        BlockSectionRouter::FwdMsgToChanMsgContextInput,
                        index,
                        msg_context.to_owned(),
                        "Context about what this message is",
                        "Add some context for why this message is being forwarded",
                    ),
                ]
                .concat()
            }
        }
    }

    fn change_selection(&mut self, type_str: &str) -> Result<()> {
        let new_action = Action::from_str(type_str)?;

        let discrim: ActionDiscriminants = self.clone().into();
        let new_action_discrim: ActionDiscriminants = new_action.into();
        if discrim == new_action_discrim {
            return Ok(());
        }

        // retain existing msg input when changing action types to save user retyping the message
        let old_string = match self {
            Action::AttachEmoji(current)
            | Action::ThreadedMessage(current)
            | Action::ChannelMessage(current) => current,
            Action::MsgOncallInThread { message, .. } => message,
            Action::ForwardMessageToChannel { msg_context, .. } => msg_context,
        };

        *self = match new_action_discrim {
            ActionDiscriminants::AttachEmoji => Action::AttachEmoji(std::mem::take(old_string)),
            ActionDiscriminants::ThreadedMessage => {
                Action::ThreadedMessage(std::mem::take(old_string))
            }
            ActionDiscriminants::ChannelMessage => {
                Action::ChannelMessage(std::mem::take(old_string))
            }
            ActionDiscriminants::MsgOncallInThread => Action::MsgOncallInThread {
                escalation_policy_id: String::default(),
                message: std::mem::take(old_string),
            },
            ActionDiscriminants::ForwardMessageToChannel => Action::ForwardMessageToChannel {
                channel: String::default(),
                msg_context: std::mem::take(old_string),
            },
        };
        // *message_action = new_action;
        Ok(())
    }

    fn validate(&self, index: Option<usize>) -> Option<SlackBlockValidationError> {
        match self {
            Action::AttachEmoji(msg_str) => msg_str.is_empty().then(|| SlackBlockValidationError {
                block_id: BlockSectionRouter::AttachEmojiInput.to_block_id(index),
                error_message: "message is empty".to_string(),
            }),
            Action::ThreadedMessage(msg_str) => {
                msg_str.is_empty().then(|| SlackBlockValidationError {
                    block_id: BlockSectionRouter::ReplyThreadedMsgInput.to_block_id(index),
                    error_message: "message is empty".to_string(),
                })
            }
            Action::ChannelMessage(msg_str) => {
                msg_str.is_empty().then(|| SlackBlockValidationError {
                    block_id: BlockSectionRouter::PostChannelMsgInput.to_block_id(index),
                    error_message: "message is empty".to_string(),
                })
            }
            Action::MsgOncallInThread {
                escalation_policy_id,
                message,
            } => {
                if message.is_empty() {
                    Some(SlackBlockValidationError {
                        block_id: BlockSectionRouter::PostChannelMsgInput.to_block_id(index),
                        error_message: "message is empty".to_string(),
                    })
                } else if escalation_policy_id.is_empty() {
                    Some(SlackBlockValidationError {
                        block_id: BlockSectionRouter::PostChannelMsgInput.to_block_id(index),
                        error_message: "no escalation policy provided".to_string(),
                    })
                } else {
                    None
                }
            }
            Action::ForwardMessageToChannel {
                channel,
                msg_context,
            } => {
                if channel.is_empty() {
                    Some(SlackBlockValidationError {
                        block_id: BlockSectionRouter::FwdMsgToChanChannelInput.to_block_id(index),
                        error_message: "Select a channel".to_string(),
                    })
                } else if msg_context.is_empty() {
                    Some(SlackBlockValidationError {
                        block_id: BlockSectionRouter::FwdMsgToChanMsgContextInput
                            .to_block_id(index),
                        error_message: "Provide some context".to_string(),
                    })
                } else {
                    None
                }
            }
        }
    }
}
