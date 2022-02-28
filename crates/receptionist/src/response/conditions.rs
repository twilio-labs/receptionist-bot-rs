use crate::{BlockSectionRouter, ReceptionistListener, SlackBlockValidationError};
use anyhow::{anyhow, bail, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use slack_morphism::prelude::*;
use std::str::FromStr;
use strum::{EnumDiscriminants, EnumIter, EnumString, IntoEnumIterator};

#[derive(Debug, Serialize, Deserialize, PartialEq, EnumDiscriminants, Clone)]
#[strum_discriminants(derive(EnumIter))]
#[serde(rename_all = "snake_case", tag = "type", content = "criteria")]
#[strum(serialize_all = "kebab_case")]
pub enum ReceptionistCondition {
    ForMessage(MessageCondition),
}

impl ReceptionistCondition {
    pub fn is_valid(&self) -> bool {
        match self {
            ReceptionistCondition::ForMessage(message_condition) => message_condition.is_valid(),
        }
    }

    pub fn default_from_listener(listener: &ReceptionistListener) -> Self {
        match listener {
            ReceptionistListener::SlackChannel { .. } => {
                Self::ForMessage(MessageCondition::MatchPhrase("".into()))
            }
        }
    }

    pub fn default_blocks(
        listener: &ReceptionistListener,
        index: Option<usize>,
    ) -> Vec<SlackBlock> {
        Self::default_from_listener(listener).to_editor_blocks(index)
    }

    pub fn to_editor_blocks(&self, index: Option<usize>) -> Vec<SlackBlock> {
        match self {
            ReceptionistCondition::ForMessage(message_condition) => {
                message_condition.to_editor_blocks(index)
            }
        }
    }

    pub fn message_phrase(phrase: &str) -> Self {
        Self::ForMessage(MessageCondition::MatchPhrase(phrase.to_string()))
    }

    pub fn iter_discriminants() -> ReceptionistConditionDiscriminantsIter {
        ReceptionistConditionDiscriminants::iter()
    }

    pub fn update_condition_type_from_action_info(
        &mut self,
        action: SlackInteractionActionInfo,
    ) -> Result<()> {
        let action_value = action
            .selected_option
            .ok_or_else(|| anyhow!("no option selected"))?
            .value;
        self.update_condition_type(&action_value)?;
        Ok(())
    }

    pub fn update_condition_type(&mut self, type_str: &str) -> Result<()> {
        match self {
            Self::ForMessage(message_condition) => {
                let mut new_variant = MessageCondition::from_str(type_str)?;
                match message_condition {
                    MessageCondition::MatchPhrase(cur_str) => {
                        new_variant.update_string(std::mem::take(cur_str))
                    }
                    MessageCondition::MatchRegex(cur_str) => {
                        new_variant.update_string(std::mem::take(cur_str))
                    }
                };
                *message_condition = new_variant;
            }
        }
        Ok(())
    }

    pub fn update_message_condition_string(&mut self, new_str: String) -> Result<()> {
        match self {
            ReceptionistCondition::ForMessage(message_condition) => {
                message_condition.update_string(new_str);
            }
            #[allow(unreachable_patterns)]
            _ => bail!("Not a message condition"),
        };
        Ok(())
    }

    pub fn validate(&self, index: Option<usize>) -> Option<SlackBlockValidationError> {
        match self {
            ReceptionistCondition::ForMessage(msg_condition) => match msg_condition {
                MessageCondition::MatchPhrase(phrase) => {
                    if phrase.is_empty() {
                        Some(SlackBlockValidationError {
                            block_id: BlockSectionRouter::MessageConditionValueInput
                                .to_block_id(index),
                            error_message: "input field is empty".to_string(),
                        })
                    } else {
                        None
                    }
                }
                MessageCondition::MatchRegex(re_str) => match Regex::new(re_str) {
                    Ok(_) => None,
                    Err(re_err) => Some(SlackBlockValidationError {
                        block_id: BlockSectionRouter::MessageConditionValueInput.to_block_id(index),
                        error_message: re_err.to_string(),
                    }),
                },
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, EnumIter, strum::Display, Clone, EnumString)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
#[strum(serialize_all = "kebab_case")]
pub enum MessageCondition {
    MatchPhrase(String),
    MatchRegex(String),
}

impl MessageCondition {
    pub fn is_valid(&self) -> bool {
        match self {
            MessageCondition::MatchPhrase(s) => !s.is_empty(),
            MessageCondition::MatchRegex(s) => !s.is_empty() && Regex::new(s).is_ok(),
        }
    }

    pub fn update_string(&mut self, new_string: String) {
        *self = match self {
            Self::MatchPhrase(_current) => Self::MatchPhrase(new_string),
            Self::MatchRegex(_current) => Self::MatchRegex(new_string),
        };
    }

    pub fn to_choice_item(&self) -> SlackBlockChoiceItem<SlackBlockPlainTextOnly> {
        SlackBlockChoiceItem::new(pt!(self.to_description()), self.to_string())
    }

    fn to_description(&self) -> &str {
        match &self {
            MessageCondition::MatchPhrase(_) => "Phrase Match",
            MessageCondition::MatchRegex(_) => "Regex Match",
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
                .with_text(md!(":clipboard: Select a match condition type"))
                .with_accessory(SlackSectionBlockElement::StaticSelect(
                    SlackBlockStaticSelectElement::new(
                        BlockSectionRouter::ConditionTypeSelected.to_action_id(index),
                        pt!("select matching Type")
                    )
                    .with_options(Self::to_choice_items())
                    .with_initial_option(self.to_choice_item())
                ))
                .with_block_id(BlockSectionRouter::ConditionTypeSelected.to_block_id(index),)
        )]
    }

    fn to_value_input_blocks(&self, index: Option<usize>) -> Vec<SlackBlock> {
        match self {
            MessageCondition::MatchPhrase(phrase) => {
                let input_element = SlackBlockPlainTextInputElement::new(
                    BlockSectionRouter::MessageConditionValueInput.to_action_id(index),
                    pt!("Phrase to match against"),
                );

                let input_element = if phrase.is_empty() {
                    input_element
                } else {
                    input_element.with_initial_value(phrase.to_owned())
                };

                slack_blocks![some_into(
                    SlackInputBlock::new(
                        pt!("Message contains this phrase:"),
                        SlackInputBlockElement::PlainTextInput(input_element)
                    )
                    .with_block_id(
                        BlockSectionRouter::MessageConditionValueInput.to_block_id(index)
                    )
                )]
            }

            MessageCondition::MatchRegex(regex_str) => {
                let input_element = SlackBlockPlainTextInputElement::new(
                    BlockSectionRouter::MessageConditionValueInput.to_action_id(index),
                    pt!("Regex pattern to match against"),
                );

                let input_element = if regex_str.is_empty() {
                    input_element
                } else {
                    input_element.with_initial_value(regex_str.to_owned())
                };

                let context: SlackContextBlockElement =
                    md!("_Tip:_ Use regex101.com to validate your syntax first :writing_hand:");

                slack_blocks![
                    some_into(
                        SlackInputBlock::new(
                            pt!("Message contains a match to this Regex pattern:"),
                            SlackInputBlockElement::PlainTextInput(input_element)
                        )
                        .with_block_id(
                            BlockSectionRouter::MessageConditionValueInput.to_block_id(index)
                        )
                    ),
                    some_into(SlackContextBlock::new(vec![context]))
                ]
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

    pub fn should_trigger(&self, message: &str) -> bool {
        match &self {
            MessageCondition::MatchPhrase(phrase) => {
                let re = Regex::new(format!("\\b{phrase}\\b").as_str())
                    .expect("Unable to compile regex for search phrase");
                re.is_match(message)
            }
            MessageCondition::MatchRegex(reg) => {
                let re = Regex::new(reg)
                    .expect("Unable to compile regex for custom regex pattern search phrase");
                re.is_match(message)
            }
        }
    }
}
