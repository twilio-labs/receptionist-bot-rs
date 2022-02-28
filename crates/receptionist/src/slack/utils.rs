use crate::{new_manager_view, MetaForManagerView};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use slack_morphism::prelude::*;
use slack_morphism_hyper::{
    SlackClientHyperConnector, SlackClientHyperHttpsConnector, SlackHyperClient,
};
use std::{collections::HashMap, env, sync::Arc};

/// Helper for slack token->client persistence
pub struct SlackStateWorkaround {
    slack_client: SlackHyperClient,
    bot_token: SlackApiToken,
}

impl SlackStateWorkaround {
    pub fn new_from_env() -> Self {
        SlackStateWorkaround {
            bot_token: SlackApiToken::new(
                std::env::var("SLACK_BOT_TOKEN")
                    .unwrap_or_else(|_| "<no_token_provided>".to_string())
                    .into(),
            ),
            slack_client: SlackClient::new(SlackClientHyperConnector::new()),
        }
    }

    pub fn open_session(&self) -> SlackClientSession<SlackClientHyperHttpsConnector> {
        self.slack_client.open_session(&self.bot_token)
    }

    pub async fn update_manager_modal_view(
        &self,
        view_id: SlackViewId,
        private_metadata: &MetaForManagerView,
    ) -> Result<()> {
        let view_update_request =
            SlackApiViewsUpdateRequest::new(new_manager_view(private_metadata).await)
                .with_view_id(view_id);

        self.open_session()
            .views_update(&view_update_request)
            .await
            .map_err(|slack_err| {
                anyhow!(
                    "Error updating existing view with meta. Error: {} | Meta: {:?}",
                    slack_err,
                    &private_metadata
                )
            })?;

        Ok(())
    }
}

pub fn setup_slack() -> Arc<SlackStateWorkaround> {
    // SETUP SHARED SLACK CLIENT
    let slack_bot_token = SlackApiToken::new(
        env::var("SLACK_BOT_TOKEN")
            .unwrap_or_else(|_| "<no_token_provided".to_string())
            .into(),
    );
    let slack_client = SlackClient::new(SlackClientHyperConnector::new());

    Arc::new(SlackStateWorkaround {
        bot_token: slack_bot_token,
        slack_client,
    })
}

/// Attributes to describe an incoming message event
pub trait MessageHelpers {
    fn is_bot_message(&self) -> bool {
        false
    }

    fn is_threaded(&self) -> bool {
        false
    }

    fn is_hidden(&self) -> bool {
        false
    }
}

impl MessageHelpers for SlackMessageEvent {
    fn is_bot_message(&self) -> bool {
        matches!(self.subtype, Some(SlackMessageEventType::BotMessage))
            || self.sender.bot_id.is_some()
    }

    fn is_threaded(&self) -> bool {
        self.origin.thread_ts.is_some()
    }

    fn is_hidden(&self) -> bool {
        self.hidden.is_some()
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "response_action", rename_all = "snake_case")]
pub enum SlackResponseAction {
    /// HashMap<SlackBlockId -> error_message>
    Errors { errors: HashMap<String, String> },
    // Update,
}

impl SlackResponseAction {
    pub fn from_validation_errors(errors: Vec<SlackBlockValidationError>) -> Self {
        let mut error_map: HashMap<String, String> = HashMap::new();

        for e in errors {
            error_map.insert(e.block_id.to_string(), e.error_message);
        }

        SlackResponseAction::Errors { errors: error_map }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SlackBlockValidationError {
    pub block_id: SlackBlockId,
    pub error_message: String,
}

/// # Examples
/// ```rust
/// use receptionist::remove_emoji_colons;
/// assert_eq!("rust", remove_emoji_colons(":rust:"));
/// assert_eq!("rust", remove_emoji_colons("rust"))
/// ```
pub fn remove_emoji_colons(emoji_name: &str) -> String {
    emoji_name.replace(":", "")
}

/// # Examples
/// ```rust
/// use receptionist::add_emoji_colons;
/// assert_eq!(":rust:", add_emoji_colons(":rust:"));
/// assert_eq!(":rust:", add_emoji_colons(":rust"));
/// assert_eq!(":rust:", add_emoji_colons("rust:"));
/// assert_eq!(":rust:", add_emoji_colons("rust"));
/// ```
pub fn add_emoji_colons(emoji_name: &str) -> String {
    match emoji_name.as_bytes() {
        [b':', .., b':'] => emoji_name.to_string(),
        [b':', ..] => format!("{emoji_name}:"),
        [.., b':'] => format!(":{emoji_name}"),
        [..] => format!(":{emoji_name}:"),
    }
}

pub fn render_channel_id(channel_id: &str) -> String {
    format!("<#{channel_id}>")
}

pub fn render_user_id(user_id: &str) -> String {
    format!("<@{user_id}>")
}

pub fn render_url_with_text(link_text: &str, url: &str) -> String {
    format!("<{url}|{link_text}>")
}

pub fn format_forwarded_message(
    origin_channel_id: &str,
    origin_user_id: &str,
    msg_permalink: &str,
    additional_msg_context: &str,
) -> String {
    let user = render_user_id(origin_user_id);
    let origin = render_channel_id(origin_channel_id);
    let msg_link = render_url_with_text("this message", msg_permalink);

    format!("{user} just sent {msg_link} to {origin}. \n _Context_: {additional_msg_context}")
}

pub fn get_sender(sender: &SlackMessageSender) -> String {
    if sender.user.is_some() {
        sender.user.clone().unwrap().to_string()
    } else if sender.bot_id.is_some() {
        sender.bot_id.clone().unwrap().to_string()
    } else {
        sender
            .username
            .clone()
            .unwrap_or_else(|| String::from("unknown user"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_emoji_colons() {
        assert_eq!("rust", remove_emoji_colons(":rust:"));
        assert_eq!("rust", remove_emoji_colons("rust"))
    }

    #[test]
    fn test_add_emoji_colons() {
        assert_eq!(":rust:", add_emoji_colons(":rust:"));
        assert_eq!(":rust:", add_emoji_colons(":rust"));
        assert_eq!(":rust:", add_emoji_colons("rust:"));
        assert_eq!(":rust:", add_emoji_colons("rust"));
    }
}
