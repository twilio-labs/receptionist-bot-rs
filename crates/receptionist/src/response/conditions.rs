use super::traits::ForListenerEvent;
use super::{listeners::ListenerEventDiscriminants, traits::SlackEditor};
use crate::{BlockSectionRouter, EnumUtils, SlackBlockValidationError};
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use slack_morphism::prelude::*;
use std::str::FromStr;
use strum::EnumString;

#[derive(EnumUtils!, Serialize, Deserialize, Clone, EnumString)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
#[strum(serialize_all = "kebab_case")]
pub enum Condition {
    MatchPhrase(String),
    MatchRegex(String),
}

impl Condition {
    pub fn update_string(&mut self, new_string: String) {
        *self = match self {
            Self::MatchPhrase(_current) => Self::MatchPhrase(new_string),
            Self::MatchRegex(_current) => Self::MatchRegex(new_string),
        };
    }

    fn is_valid(&self) -> bool {
        match self {
            Condition::MatchPhrase(s) => !s.is_empty(),
            Condition::MatchRegex(s) => !s.is_empty() && Regex::new(s).is_ok(),
        }
    }

    pub fn should_trigger(&self, message: &str) -> bool {
        match &self {
            Condition::MatchPhrase(phrase) => {
                let re = Regex::new(format!("\\b{phrase}\\b").as_str())
                    .expect("Unable to compile regex for search phrase");
                re.is_match(message)
            }
            Condition::MatchRegex(reg) => {
                let re = Regex::new(reg)
                    .expect("Unable to compile regex for custom regex pattern search phrase");
                re.is_match(message)
            }
        }
    }
}

impl ForListenerEvent for Condition {
    fn listeners(&self) -> Vec<ListenerEventDiscriminants> {
        match self {
            Condition::MatchPhrase(_) => vec![ListenerEventDiscriminants::SlackChannelMessage],
            Condition::MatchRegex(_) => vec![ListenerEventDiscriminants::SlackChannelMessage],
        }
    }

    fn default_from_listener(listener: &ListenerEventDiscriminants) -> Self {
        match listener {
            ListenerEventDiscriminants::SlackChannelMessage => Self::MatchPhrase(String::default()),
        }
    }
}

impl SlackEditor for Condition {
    fn to_description(&self) -> &str {
        match &self {
            Condition::MatchPhrase(_) => "Phrase Match",
            Condition::MatchRegex(_) => "Regex Match",
        }
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
            Condition::MatchPhrase(phrase) => {
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

            Condition::MatchRegex(regex_str) => {
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

    fn change_selection(&mut self, type_str: &str) -> Result<()> {
        let mut new_variant = Condition::from_str(type_str)?;
        match self {
            Condition::MatchPhrase(cur_str) => new_variant.update_string(std::mem::take(cur_str)),
            Condition::MatchRegex(cur_str) => new_variant.update_string(std::mem::take(cur_str)),
        };
        *self = new_variant;

        Ok(())
    }

    fn validate(&self, index: Option<usize>) -> Option<SlackBlockValidationError> {
        match self {
            Condition::MatchPhrase(phrase) => {
                if phrase.is_empty() {
                    Some(SlackBlockValidationError {
                        block_id: BlockSectionRouter::MessageConditionValueInput.to_block_id(index),
                        error_message: "input field is empty".to_string(),
                    })
                } else {
                    None
                }
            }
            Condition::MatchRegex(re_str) => match Regex::new(re_str) {
                Ok(_) => None,
                Err(re_err) => Some(SlackBlockValidationError {
                    block_id: BlockSectionRouter::MessageConditionValueInput.to_block_id(index),
                    error_message: re_err.to_string(),
                }),
            },
        }
    }
}
