#[cfg(any(feature = "tempdb", feature = "dynamodb"))]
use crate::database::get_responses_for_collaborator;
use crate::{get_response_by_id, BlockSectionRouter, EnumUtils, ReceptionistResponse};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::to_string;
use slack_morphism::prelude::*;
use std::str::FromStr;
use strum::{EnumString, IntoEnumIterator};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct MetaForManagerView {
    pub current_mode: ManagerViewMode,
    pub user_id: String,
    pub existing_response_id: Option<String>,
    pub response: Option<ReceptionistResponse>,
}

impl MetaForManagerView {
    pub fn new(current_mode: ManagerViewMode, user_id: String) -> Self {
        Self {
            current_mode,
            user_id,
            response: None,
            existing_response_id: None,
        }
    }
}

impl Default for MetaForManagerView {
    fn default() -> Self {
        Self {
            current_mode: ManagerViewMode::Home,
            response: None,
            existing_response_id: None,
            user_id: "".to_string(),
        }
    }
}

pub fn select_mode(
    mode_str_value: &str,
    metadata: &MetaForManagerView,
) -> Result<MetaForManagerView> {
    if let Ok(mode) = ManagerViewMode::from_str(mode_str_value) {
        Ok(MetaForManagerView::new(mode, metadata.user_id.to_owned()))
    } else {
        bail!("unable to select mode");
    }
}

fn manager_view_wrapper(blocks: Vec<SlackBlock>, meta: &MetaForManagerView) -> SlackView {
    SlackView::Modal(
        SlackModalView::new("Receptionist Manager".into(), blocks)
            .opt_submit(Some("Submit".into()))
            .opt_close(Some("Close Manager".into()))
            .with_private_metadata(to_string(&meta).expect("unable to serialize private meta")),
    )
}

pub async fn new_manager_view(meta: &MetaForManagerView) -> SlackView {
    let mut blocks: Vec<SlackBlock> = meta.current_mode.to_editor_blocks();

    let extra_blocks = match &meta.current_mode {
        ManagerViewMode::Home => Vec::default(),
        ManagerViewMode::CreateResponse => ReceptionistResponse::default().to_editor_blocks(),
        ManagerViewMode::EditResponse => {
            let mut editing_blocks = response_selector_blocks(&meta.user_id).await;

            if let Some(response_id) = &meta.existing_response_id {
                let response_to_edit = get_response_by_id(response_id).await;
                editing_blocks.extend(
                    response_to_edit
                        .expect(&format!(
                            "cannot edit response that does not exist. id: {response_id}"
                        ))
                        .to_editor_blocks(),
                )
            }
            editing_blocks
        }
        ManagerViewMode::DeleteResponse => response_selector_blocks(&meta.user_id).await,
    };

    blocks.extend(extra_blocks);
    manager_view_wrapper(blocks, meta)
}

async fn response_selector_blocks(user_id: &str) -> Vec<SlackBlock> {
    let responses = get_responses_for_collaborator(user_id)
        .await
        .expect("error getting responses");

    if responses.is_empty() {
        return slack_blocks![some_into(SlackSectionBlock::new().with_text(pt!(
            "You are not collaborator on any responses, please create a new response or ask another user to add you to an existing response."
        )))];
    }

    let options: Vec<SlackBlockChoiceItem<SlackBlockPlainTextOnly>> = responses
        .iter()
        .map(|res| res.to_response_choice_item())
        .collect();

    let static_selector = SlackBlockStaticSelectElement::new(
        BlockSectionRouter::ResponseSelection.to_action_id(None),
        pt!("Select one of your existing Responses"),
    )
    .with_options(options);

    slack_blocks![
        some_into(
            SlackInputBlock::new(
                SlackBlockPlainTextOnly::from("Select one of your existing Responses"),
                SlackInputBlockElement::StaticSelect(static_selector)
            )
            .with_dispatch_action(true)
            .without_optional()
            .with_block_id(BlockSectionRouter::ResponseSelection.to_block_id(None))
        ),
        some_into(SlackDividerBlock::new())
    ]
}

#[derive(EnumUtils!, EnumString, Serialize, Deserialize, Clone)]
#[strum(serialize_all = "kebab_case")]
pub enum ManagerViewMode {
    Home,
    CreateResponse,
    EditResponse,
    DeleteResponse,
}

impl Default for ManagerViewMode {
    fn default() -> Self {
        Self::Home
    }
}

impl ManagerViewMode {
    fn to_choice_item(&self) -> SlackBlockChoiceItem<SlackBlockPlainTextOnly> {
        let description = match &self {
            ManagerViewMode::Home => "Management Home",
            ManagerViewMode::CreateResponse => "Create a Receptionist Response",
            ManagerViewMode::EditResponse => "Edit an existing Response",
            ManagerViewMode::DeleteResponse => "Delete an existing Response",
        };

        SlackBlockChoiceItem::new(pt!(description), self.to_string())
    }

    fn to_editor_blocks(&self) -> Vec<SlackBlock> {
        let options: Vec<SlackBlockChoiceItem<SlackBlockPlainTextOnly>> = ManagerViewMode::iter()
            .map(|management_type| management_type.to_choice_item())
            .collect();

        let static_selector = SlackBlockStaticSelectElement::new(
            BlockSectionRouter::ManagerModeSelection.to_action_id(None),
            pt!("Select a function"),
        )
        .with_options(options)
        .with_initial_option(self.to_choice_item());

        slack_blocks![
            some_into(
                SlackInputBlock::new(
                    SlackBlockPlainTextOnly::from("What would you like to do?"),
                    SlackInputBlockElement::StaticSelect(static_selector)
                )
                .with_dispatch_action(true)
                .without_optional()
                .with_block_id(BlockSectionRouter::ManagerModeSelection.to_block_id(None))
            ),
            some_into(SlackDividerBlock::new())
        ]
    }
}
