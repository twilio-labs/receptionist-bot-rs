use std::fmt::{self};

// use derive_alias::derive_alias;
use serde::{Deserialize, Serialize};
use slack_morphism::prelude::*;
use strum::{Display, EnumDiscriminants, EnumIter, EnumString, IntoEnumIterator};

// Generates a macro (`derive_cmp`) that will attach the listed derives to a given item
derive_alias! {
    #[derive(EnumUtils!)] = #[derive(strum::Display, strum::EnumDiscriminants, strum::EnumIter, strum::EnumString, Debug, serde::Serialize, serde::Deserialize, PartialEq)];
    #[derive(Serde!)] = #[derive(serde::Serialize, serde::Deserialize)];
}

pub trait SlackEditor: fmt::Display + IntoEnumIterator {
    fn to_description(&self) -> &str;

    fn to_type_selector_blocks(&self, index: Option<usize>) -> Vec<SlackBlock>;

    fn to_value_input_blocks(&self, index: Option<usize>) -> Vec<SlackBlock>;

    fn to_editor_blocks(&self, index: Option<usize>) -> Vec<SlackBlock>;

    fn to_choice_item(&self) -> SlackBlockChoiceItem<SlackBlockPlainTextOnly> {
        SlackBlockChoiceItem::new(pt!(self.to_description()), self.to_string())
    }

    fn to_choice_items() -> Vec<SlackBlockChoiceItem<SlackBlockPlainTextOnly>> {
        Self::iter()
            .map(|variant| variant.to_choice_item())
            .collect()
    }
}

pub trait ForListenerEvent {
    fn listeners(&self) -> Vec<ListenerEventDiscriminants>;
}

#[derive(EnumUtils!)]
pub enum ListenerEvent {
    SlackChannelMessage,
    SlackSlashCommand,
}

#[derive(EnumUtils!)]
pub enum Action {
    AttachEmoji(String),
    /// Post message in thread of the triggered message
    ThreadedMessage(String),
    // /// Send message to same channel that triggered message
    // ChannelMessage(String),
    // MsgOncallInThread {
    //     escalation_policy_id: String,
    //     message: String,
    // },
    // /// Forward the triggered message to a different channel
    // ForwardMessageToChannel {
    //     channel: String,
    //     msg_context: String,
    // },
}

impl SlackEditor for Action {
    fn to_description(&self) -> &str {
        match self {
            Action::AttachEmoji(_) => "Attach Emoji to Message",
            Action::ThreadedMessage(_) => "Reply with Threaded Message",
            // Action::ChannelMessage(_) => "Post Message to Same Channel",
            // Action::MsgOncallInThread { .. } => "Tag OnCall User in Thread",
            // Action::ForwardMessageToChannel { .. } => {
            //     "Forward detected message to a different channel"
        }
    }

    fn to_type_selector_blocks(&self, index: Option<usize>) -> Vec<SlackBlock> {
        todo!()
    }

    fn to_value_input_blocks(&self, index: Option<usize>) -> Vec<SlackBlock> {
        todo!()
    }

    fn to_editor_blocks(&self, index: Option<usize>) -> Vec<SlackBlock> {
        todo!()
    }
}
