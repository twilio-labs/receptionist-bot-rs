use super::listeners::ListenerEventDiscriminants;
use crate::SlackBlockValidationError;
use anyhow::{anyhow, Result};
use slack_morphism::prelude::*;
use std::fmt;
use strum::IntoEnumIterator;

pub trait SlackEditor: fmt::Display + IntoEnumIterator {
    fn to_description(&self) -> &str;

    fn to_type_selector_blocks(&self, index: Option<usize>) -> Vec<SlackBlock>;

    fn to_value_input_blocks(&self, index: Option<usize>) -> Vec<SlackBlock>;

    fn validate(&self, index: Option<usize>) -> Option<SlackBlockValidationError>;

    fn change_selection(&mut self, type_str: &str) -> Result<()>;

    fn change_selection_from_action_info(
        &mut self,
        action: SlackInteractionActionInfo,
    ) -> Result<()> {
        let action_value = action
            .selected_option
            .ok_or_else(|| anyhow!("no option selected"))?
            .value;
        self.change_selection(&action_value)?;
        Ok(())
    }

    fn to_editor_blocks(&self, index: Option<usize>) -> Vec<SlackBlock> {
        [
            self.to_type_selector_blocks(index),
            self.to_value_input_blocks(index),
            vec![SlackDividerBlock::new().into()],
        ]
        .concat()
    }

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

    fn default_from_listener(listener: &ListenerEventDiscriminants) -> Self;
}
