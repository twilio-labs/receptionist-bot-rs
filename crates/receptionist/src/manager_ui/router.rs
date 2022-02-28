use anyhow::{anyhow, Context, Result};
use slack_morphism::prelude::*;
use strum::{EnumDiscriminants, EnumIter, EnumString, IntoEnumIterator};

/// Route nested action strings to their appropriate handler
#[derive(Debug, EnumDiscriminants, EnumString, EnumIter, strum::Display)]
#[strum(serialize_all = "kebab_case")]
pub enum BlockSectionRouter {
    /// single inputs section
    ManagerModeSelection,
    ResponseSelection,
    CollaboratorSelection,

    // Listener Section
    ListenerChannelSelected,

    // Condition Section
    ConditionTypeSelected,
    MessageConditionValueInput,

    // Action Section
    ActionTypeSelected,
    AttachEmojiInput,
    ReplyThreadedMsgInput,
    PostChannelMsgInput,
    PDEscalationPolicyInput,
    PDThreadedMsgInput,
    FwdMsgToChanChannelInput,
    FwdMsgToChanMsgContextInput,
}

impl BlockSectionRouter {
    fn index_delimiter() -> &'static str {
        "_IDX_"
    }

    pub fn find_route(action_id: &str) -> Option<Self> {
        for variant in Self::iter() {
            if action_id.starts_with(&variant.to_string()) {
                return Some(variant);
            }
        }
        None
    }

    pub fn from_action_id(action_id: &SlackActionId) -> Option<Self> {
        let action_ref = action_id.as_ref();
        for variant in Self::iter() {
            if action_ref.starts_with(&variant.to_string()) {
                return Some(variant);
            }
        }
        None
    }

    pub fn from_action_id_with_index(action_id: &SlackActionId) -> Option<(Self, Result<usize>)> {
        let action_ref = action_id.as_ref();
        for variant in Self::iter() {
            if action_ref.starts_with(&variant.to_string()) {
                let index_result = Self::get_index_from_action_id_str(action_ref);
                return Some((variant, index_result));
            }
        }
        None
    }

    pub fn from_string_with_index(action_id_str: &str) -> Option<(Self, Result<usize>)> {
        for variant in Self::iter() {
            if action_id_str.starts_with(&variant.to_string()) {
                let index_result = Self::get_index_from_action_id_str(action_id_str);
                return Some((variant, index_result));
            }
        }
        None
    }

    pub fn to_string_with_index(&self, index: Option<usize>) -> String {
        self.to_string() + Self::index_delimiter() + &index.unwrap_or_default().to_string()
    }

    pub fn to_action_id(&self, index: Option<usize>) -> SlackActionId {
        self.to_string_with_index(index).into()
    }

    pub fn to_block_id(&self, index: Option<usize>) -> SlackBlockId {
        format!("BLOCK-{}", self.to_string_with_index(index)).into()
    }

    pub fn get_index_from_action_id_str(action_id: &str) -> Result<usize> {
        let (_prefix, suffix) = action_id
            .split_once(Self::index_delimiter())
            .ok_or_else(|| anyhow!("index delimiter not found in action_id"))?;

        suffix
            .parse::<usize>()
            .with_context(|| "failed to parse index from action id")
    }
}
