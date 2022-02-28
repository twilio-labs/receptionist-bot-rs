use receptionist::{
    write_serde_struct_to_file, MessageAction, MessageCondition, ReceptionistAction,
    ReceptionistCondition, ReceptionistResponse,
};
use serde::{Deserialize, Serialize};
use slack_morphism::prelude::*;
use std::path::{Path, PathBuf};

const GENERATED_BLOCKS_DIR: &str = "tests/generated_blocks";
#[derive(Serialize, Deserialize)]
struct BlockKitTemplate {
    blocks: Vec<SlackBlock>,
}

impl From<ReceptionistResponse> for BlockKitTemplate {
    fn from(rec_response: ReceptionistResponse) -> Self {
        Self {
            blocks: rec_response.to_editor_blocks(),
        }
    }
}

fn build_generated_path(file_name: &str) -> PathBuf {
    let formatted_file = if file_name.ends_with(".json") {
        file_name.to_string()
    } else {
        format!("{file_name}.json")
    };

    Path::new(GENERATED_BLOCKS_DIR).join(formatted_file)
}

fn write_preview_file(name: &str, rec_response: ReceptionistResponse) {
    write_serde_struct_to_file(
        build_generated_path(&format!("{name}.json")),
        BlockKitTemplate::from(rec_response),
    )
}

#[test]
fn gen_action_attach_emoji() {
    let mut rec_response = ReceptionistResponse::default();

    let action = rec_response.actions.first_mut().unwrap();
    *action = ReceptionistAction::ForMessage(MessageAction::AttachEmoji(":thumbsup:".into()));

    write_preview_file("attach_emoji", rec_response)
}

#[test]
fn gen_action_tag_oncall() {
    let mut rec_response = ReceptionistResponse::default();

    let action = rec_response.actions.first_mut().unwrap();
    *action = ReceptionistAction::ForMessage(MessageAction::MsgOncallInThread {
        escalation_policy_id: "some_id".into(),
        message: "some_message".into(),
    });

    write_preview_file("tag_oncall_in_thread", rec_response)
}

#[test]
fn gen_action_forward_msg_to_channel() {
    let mut rec_response = ReceptionistResponse::default();

    let action = rec_response.actions.first_mut().unwrap();
    *action = ReceptionistAction::ForMessage(MessageAction::ForwardMessageToChannel {
        channel: "".into(),
        msg_context: "some_message".into(),
    });

    write_preview_file("fwd_msg_to_channel", rec_response)
}

#[test]
fn gen_condition_match_regex() {
    let mut rec_response = ReceptionistResponse::default();

    let condition = rec_response.conditions.first_mut().unwrap();
    *condition =
        ReceptionistCondition::ForMessage(MessageCondition::MatchRegex("<my_regex>".into()));

    write_preview_file("match_regex", rec_response)
}
