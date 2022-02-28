use crate::{
    response::utils::slack_plain_text_input_block_for_view, BlockSectionRouter,
    ReceptionistListener, SlackBlockValidationError,
};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use slack_morphism::prelude::*;
use std::str::FromStr;
use strum::{Display, EnumDiscriminants, EnumIter, EnumString, IntoEnumIterator};

#[derive(Debug, Serialize, Deserialize, PartialEq, EnumDiscriminants, Clone)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum ReceptionistAction {
    ForMessage(MessageAction),
}

impl ReceptionistAction {
    pub fn validate(&self, index: Option<usize>) -> Option<SlackBlockValidationError> {
        match self {
            ReceptionistAction::ForMessage(msg_action) => match msg_action {
                MessageAction::AttachEmoji(msg_str) => {
                    msg_str.is_empty().then(|| SlackBlockValidationError {
                        block_id: BlockSectionRouter::AttachEmojiInput.to_block_id(index),
                        error_message: "message is empty".to_string(),
                    })
                }
                MessageAction::ThreadedMessage(msg_str) => {
                    msg_str.is_empty().then(|| SlackBlockValidationError {
                        block_id: BlockSectionRouter::ReplyThreadedMsgInput.to_block_id(index),
                        error_message: "message is empty".to_string(),
                    })
                }
                MessageAction::ChannelMessage(msg_str) => {
                    msg_str.is_empty().then(|| SlackBlockValidationError {
                        block_id: BlockSectionRouter::PostChannelMsgInput.to_block_id(index),
                        error_message: "message is empty".to_string(),
                    })
                }
                MessageAction::MsgOncallInThread {
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
                MessageAction::ForwardMessageToChannel {
                    channel,
                    msg_context,
                } => {
                    if channel.is_empty() {
                        Some(SlackBlockValidationError {
                            block_id: BlockSectionRouter::FwdMsgToChanChannelInput
                                .to_block_id(index),
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
            },
        }
    }

    pub fn default_from_listener(listener: &ReceptionistListener) -> Self {
        match listener {
            ReceptionistListener::SlackChannel { .. } => {
                Self::ForMessage(MessageAction::AttachEmoji("".to_string()))
            }
        }
    }

    pub fn default_blocks(
        listener: &ReceptionistListener,
        index: Option<usize>,
    ) -> Vec<SlackBlock> {
        Self::default_from_listener(listener).to_editor_blocks(index)
    }

    pub fn to_choice_items(&self) -> Vec<SlackBlockChoiceItem<SlackBlockPlainTextOnly>> {
        match self {
            ReceptionistAction::ForMessage(..) => MessageAction::to_choice_items(),
        }
    }

    pub fn to_editor_blocks(&self, index: Option<usize>) -> Vec<SlackBlock> {
        match self {
            ReceptionistAction::ForMessage(message_action) => {
                message_action.to_editor_blocks(index)
            }
        }
    }

    pub fn update_action_type_from_action_info(
        &mut self,
        action: SlackInteractionActionInfo,
    ) -> Result<()> {
        let action_type_str = action
            .selected_option
            .ok_or_else(|| anyhow!("no option selected"))?
            .value;
        self.update_action_type(&action_type_str)?;
        Ok(())
    }

    pub fn update_action_type(&mut self, type_str: &str) -> Result<()> {
        match self {
            Self::ForMessage(message_action) => {
                let new_action = MessageAction::from_str(type_str)?;

                let discrim: MessageActionDiscriminants = message_action.clone().into();
                let new_action_discrim: MessageActionDiscriminants = new_action.into();
                if discrim == new_action_discrim {
                    return Ok(());
                }

                // retain existing msg input when changing action types to save user retyping the message
                let old_string = match message_action {
                    MessageAction::AttachEmoji(current)
                    | MessageAction::ThreadedMessage(current)
                    | MessageAction::ChannelMessage(current) => current,
                    MessageAction::MsgOncallInThread { message, .. } => message,
                    MessageAction::ForwardMessageToChannel { msg_context, .. } => msg_context,
                };

                *message_action = match new_action_discrim {
                    MessageActionDiscriminants::AttachEmoji => {
                        MessageAction::AttachEmoji(std::mem::take(old_string))
                    }
                    MessageActionDiscriminants::ThreadedMessage => {
                        MessageAction::ThreadedMessage(std::mem::take(old_string))
                    }
                    MessageActionDiscriminants::ChannelMessage => {
                        MessageAction::ChannelMessage(std::mem::take(old_string))
                    }
                    MessageActionDiscriminants::MsgOncallInThread => {
                        MessageAction::MsgOncallInThread {
                            escalation_policy_id: String::default(),
                            message: std::mem::take(old_string),
                        }
                    }
                    MessageActionDiscriminants::ForwardMessageToChannel => {
                        MessageAction::ForwardMessageToChannel {
                            channel: String::default(),
                            msg_context: std::mem::take(old_string),
                        }
                    }
                };

                // *message_action = new_action;
            }
        };
        Ok(())
    }
}

#[derive(
    Debug,
    Serialize,
    Deserialize,
    PartialEq,
    EnumIter,
    EnumString,
    Display,
    Clone,
    EnumDiscriminants,
)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
#[strum(serialize_all = "kebab_case")]
pub enum MessageAction {
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

impl MessageAction {
    pub fn to_choice_item(&self) -> SlackBlockChoiceItem<SlackBlockPlainTextOnly> {
        SlackBlockChoiceItem::new(pt!(self.to_description()), self.to_string())
    }

    pub fn to_description(&self) -> &str {
        match &self {
            MessageAction::AttachEmoji(_) => "Attach Emoji to Message",
            MessageAction::ThreadedMessage(_) => "Reply with Threaded Message",
            MessageAction::ChannelMessage(_) => "Post Message to Same Channel",
            MessageAction::MsgOncallInThread { .. } => "Tag OnCall User in Thread",
            MessageAction::ForwardMessageToChannel { .. } => {
                "Forward detected message to a different channel"
            }
        }
    }

    pub fn to_choice_items() -> Vec<SlackBlockChoiceItem<SlackBlockPlainTextOnly>> {
        Self::iter()
            .map(|variant| variant.to_choice_item())
            .collect()
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
            MessageAction::AttachEmoji(emoji) => slack_plain_text_input_block_for_view(
                BlockSectionRouter::AttachEmojiInput,
                index,
                emoji.to_owned(),
                "my-emoji",
                "Choose an emoji (can also trigger Slack Workflows)",
            ),
            MessageAction::ThreadedMessage(msg) => slack_plain_text_input_block_for_view(
                BlockSectionRouter::ReplyThreadedMsgInput,
                index,
                msg.to_owned(),
                "It looks like you're looking for..",
                "Enter a message to post in thread",
            ),
            MessageAction::ChannelMessage(msg) => slack_plain_text_input_block_for_view(
                BlockSectionRouter::PostChannelMsgInput,
                index,
                msg.to_owned(),
                "Hey Channel..",
                "Enter Message to Post in Channel (not thread)",
            ),
            MessageAction::MsgOncallInThread {
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
            MessageAction::ForwardMessageToChannel {
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

    pub fn to_editor_blocks(&self, index: Option<usize>) -> Vec<SlackBlock> {
        [
            self.to_type_selector_blocks(index),
            self.to_value_input_blocks(index),
            vec![SlackDividerBlock::new().into()],
        ]
        .concat()
    }
}
