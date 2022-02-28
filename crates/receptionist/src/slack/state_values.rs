use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use slack_morphism::prelude::*;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct CustomSlackViewState(HashMap<String, serde_json::Value>);

#[derive(Serialize, Deserialize, Debug)]
pub struct ViewBlockState(HashMap<String, ViewBlockStateType>);

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ViewBlockStateType {
    ConversationsSelect {
        selected_conversation: Option<SlackConversationId>,
    },
    StaticSelect {
        selected_option: StaticSelectSelectedOptionValueState,
    },
    PlainTextInput {
        value: String,
    },
    MultiUsersSelect {
        selected_users: Vec<String>,
    },
}

impl ViewBlockStateType {
    pub fn get_value_from_static_select(&self) -> Result<String> {
        if let ViewBlockStateType::StaticSelect { selected_option } = self {
            Ok(selected_option.value.to_owned())
        } else {
            bail!("block is not static_select")
        }
    }

    pub fn get_conversation_select_value(&self) -> Result<Option<SlackConversationId>> {
        match self {
            ViewBlockStateType::ConversationsSelect {
                selected_conversation,
            } => Ok(selected_conversation.to_owned()),
            _ => bail!("block is not conversation_select"),
        }
    }

    pub fn get_plain_text_value(&self) -> Result<String> {
        match self {
            ViewBlockStateType::PlainTextInput { value } => Ok(value.to_owned()),
            _ => bail!("block is not a plain text input"),
        }
    }

    pub fn get_multi_users_select_value(&self) -> Result<Vec<String>> {
        match self {
            ViewBlockStateType::MultiUsersSelect { selected_users } => {
                Ok(selected_users.to_owned())
            }
            _ => bail!("block is not a multi_users_select input"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StaticSelectSelectedOptionValueState {
    pub text: serde_json::Value,
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_value, json};

    #[test]
    fn test_custom_slack_value_types() {
        let test = json!({
        "BLOCK-channel-select_IDX_0": {
            "channel-select_IDX_0": {
                "selected_conversation": "G01EVFPD63V",
                "type": "conversations_select",
            }
        }});

        let test_2 = json!({
            "selected_conversation": "G01EVFPD63V",
            "type": "conversations_select",
        });

        let _view_block_state_type: ViewBlockStateType = from_value(test_2).unwrap();
        let _custom_slack_view_state_wrapper: CustomSlackViewState = from_value(test).unwrap();
    }
}
