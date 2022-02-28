use super::SlackStateWorkaround;

#[cfg(any(feature = "tempdb", feature = "dynamodb"))]
use crate::database::get_responses_for_listener;
use crate::{
    config::get_or_init_app_config,
    format_forwarded_message, get_sender,
    response::{MessageAction, ReceptionistAction, ReceptionistCondition, ReceptionistResponse},
    slack::api_calls::reactions_add,
    MessageHelpers, ReceptionistListener,
};
use axum::{extract::Extension, http::StatusCode, response::IntoResponse, Json};
use serde_json::{to_value, Value};
use slack_morphism::prelude::*;
use std::sync::Arc;
use tracing::error;

pub async fn axum_handler_slack_events_api(
    Extension(slack_state): Extension<Arc<SlackStateWorkaround>>,
    Json(payload): Json<SlackPushEvent>,
) -> impl IntoResponse {
    let response = handle_slack_event(&*slack_state, payload).await;
    (response.0, Json(response.1))
}

pub async fn handle_slack_event(
    slack_state: &SlackStateWorkaround,
    payload: SlackPushEvent,
) -> (StatusCode, Value) {
    match payload {
        SlackPushEvent::EventCallback(event_req) => {
            let response_body =
                process_event_callback_for_receptionist(event_req, slack_state).await;
            (StatusCode::OK, response_body)
        }
        SlackPushEvent::UrlVerification(url_verify_req) => {
            (StatusCode::OK, to_value(url_verify_req).unwrap())
        }
        SlackPushEvent::AppRateLimited(rate_limit_req) => {
            // TODO: handle rate limits
            (StatusCode::OK, to_value(rate_limit_req).unwrap())
        }
    }
}

pub async fn process_event_callback_for_receptionist(
    event_req: SlackPushEventCallback,
    // slack_client: Arc<SlackStateWorkaround>,
    slack_client: &SlackStateWorkaround,
) -> Value {
    let default_event_response = Value::default();
    match event_req.event {
        SlackEventCallbackBody::Message(event) => {
            if [
                event.is_bot_message(),
                event.is_hidden(),
                event.is_threaded(),
            ]
            .iter()
            .any(|x| *x)
            {
                return default_event_response;
            }

            let message_content = event.content.unwrap().text.unwrap();
            let event_channel_id = event
                .origin
                .channel
                .unwrap_or_else(|| SlackChannelId("".to_string()));

            let responses_for_channel_id =
                get_responses_for_listener(ReceptionistListener::SlackChannel {
                    channel_id: event_channel_id.to_string(),
                })
                .await
                .expect("unable to get responses for channel");

            let responses_for_message_type: Vec<ReceptionistResponse> = responses_for_channel_id
                .iter()
                .filter(|r| {
                    r.conditions
                        .iter()
                        .any(|t_type| matches!(&t_type, &ReceptionistCondition::ForMessage(_)))
                })
                .map(|r| r.to_owned())
                .collect();

            let slack_session = slack_client.open_session();

            for rec_response in responses_for_message_type {
                if rec_response.check_for_match(&message_content) {
                    for action in &rec_response.actions {
                        match &action {
                            ReceptionistAction::ForMessage(message_action) => {
                                match message_action {
                                    MessageAction::AttachEmoji(name) => {
                                        if let Err(slack_err) = reactions_add(
                                            &slack_session,
                                            &event_channel_id.to_string(),
                                            event.origin.ts.as_ref(),
                                            name,
                                        )
                                        .await
                                        {
                                            // log error
                                            error!("{}", slack_err);
                                        }
                                    }
                                    MessageAction::ThreadedMessage(msg) => {
                                        if let Err(slack_err) = slack_session
                                            .chat_post_message(
                                                &SlackApiChatPostMessageRequest::new(
                                                    event_channel_id.to_owned(),
                                                    SlackMessageContent::new()
                                                        .with_text(msg.to_owned()),
                                                )
                                                .with_thread_ts(event.origin.ts.to_owned()),
                                            )
                                            .await
                                        {
                                            error!("{}", slack_err);
                                        }
                                    }
                                    MessageAction::ChannelMessage(msg) => {
                                        if let Err(slack_err) = slack_session
                                            .chat_post_message(
                                                &SlackApiChatPostMessageRequest::new(
                                                    event_channel_id.to_owned(),
                                                    SlackMessageContent::new()
                                                        .with_text(msg.to_owned()),
                                                ),
                                            )
                                            .await
                                        {
                                            error!("{}", slack_err);
                                        }
                                    }
                                    MessageAction::MsgOncallInThread {
                                        escalation_policy_id,
                                        message,
                                    } => {
                                        // get oncall user
                                        match &get_or_init_app_config().await.pagerduty_config {
                                            Some(pd) => {
                                                match  pd.get_oncalls(escalation_policy_id.to_owned()).await {
                                                    Ok(oncalls_list) => {
                                                        if let Some(pd_user) = oncalls_list.oncalls.first() {
                                                            let slack_user = slack_session.users_lookup_by_email(&SlackApiUsersLookupByEmailRequest::new(pd_user.user.email.clone().into())).await;
                                                            match slack_user {
                                                                Ok(slack_profile) => {
                                                                    if let Err(slack_err) = slack_session.chat_post_message(
                                                                        &SlackApiChatPostMessageRequest::new(
                                                                            event_channel_id.to_owned(),
                                                                            SlackMessageContent::new()
                                                                                .with_text(format!("<@{}> - {message}", slack_profile.user.id)),
                                                                        )
                                                                        .with_thread_ts(event.origin.ts.to_owned()),
                                                                    )
                                                                    .await {
                                                                        error!("Error posting to thread: {}", slack_err)
                                                                    }

                                                                },
                                                                Err(slack_err) => error!("Unable to get slack profile for PD user - {}", slack_err), 
                                                            }
                                                        }
                                                    },
                                                    Err(err) => error!("Error fetching oncalls from pd for escalation policy {} - {}", escalation_policy_id, err),
                                                }

                                            },
                                            None => error!("No pagerduty token configured, unable to tag user in thread"),
                                        }
                                    }
                                    MessageAction::ForwardMessageToChannel {
                                        channel,
                                        msg_context,
                                    } => {
                                        match slack_session
                                            .chat_get_permalink(
                                                &SlackApiChatGetPermalinkRequest::new(
                                                    event_channel_id.to_owned(),
                                                    event.origin.ts.to_owned(),
                                                ),
                                            )
                                            .await
                                        {
                                            Ok(permalink_resp) => {
                                                let permalink = permalink_resp.permalink;
                                                let sender = get_sender(&event.sender);
                                                if let Err(slack_err) = slack_session
                                                    .chat_post_message(
                                                        &SlackApiChatPostMessageRequest::new(
                                                            channel.into(),
                                                            SlackMessageContent::new().with_text(
                                                                format_forwarded_message(
                                                                    event_channel_id.as_ref(),
                                                                    &sender,
                                                                    &permalink.to_string(),
                                                                    msg_context,
                                                                ),
                                                            ),
                                                        ),
                                                    )
                                                    .await
                                                {
                                                    error!(
                                                        "Failed to forward message {}",
                                                        slack_err
                                                    );
                                                }
                                            }
                                            Err(slack_err) => error!(
                                                "Failed to get permalink to forward message: {}",
                                                slack_err
                                            ),
                                        };
                                    }
                                }
                            }
                        };
                    }
                }
            }
        }
        _ => todo!(),
    }

    default_event_response
}
