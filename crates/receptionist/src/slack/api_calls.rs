/// These should be merged upstream to slack-morphism if possible
///
use serde::{Deserialize, Serialize};
use slack_morphism::{ClientResult, SlackClientSession};
use slack_morphism_hyper::SlackClientHyperHttpsConnector;
use slack_morphism_models::{SlackChannelId, SlackTs};

pub async fn reactions_add(
    slack_session: &SlackClientSession<'_, SlackClientHyperHttpsConnector>,
    channel: &str,
    timestamp: &str,
    name: &str,
) -> ClientResult<SlackApiReactionsAddResponse> {
    slack_session
        .http_session_api
        .http_post(
            "reactions.add",
            &SlackApiReactionsAddRequest {
                channel: channel.to_owned().into(),
                name: name.to_owned(),
                timestamp: timestamp.to_owned().into(),
            },
            None,
        )
        .await
}

// #[skip_serializing_none]
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct SlackApiReactionsAddRequest {
    pub channel: SlackChannelId,
    pub timestamp: SlackTs,
    pub name: String,
}

// #[skip_serializing_none]
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct SlackApiReactionsAddResponse {}
