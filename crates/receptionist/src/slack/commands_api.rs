use super::SlackStateWorkaround;
use crate::manager_ui::{new_manager_view, ManagerViewMode, MetaForManagerView};
use axum::{
    extract::{Extension, Form},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::{to_value, Value};
use slack_morphism::prelude::*;
use std::sync::Arc;
use tracing::error;

pub async fn axum_handler_handle_slack_commands_api(
    Extension(slack_state): Extension<Arc<SlackStateWorkaround>>,
    Form(payload): Form<SlackCommandEvent>,
) -> impl IntoResponse {
    let response = handle_slack_command(&*slack_state, payload).await;
    (response.0, Json(response.1))
}

pub async fn handle_slack_command(
    slack_state: &SlackStateWorkaround,
    payload: SlackCommandEvent,
) -> (StatusCode, Value) {
    let view = new_manager_view(&MetaForManagerView::new(
        ManagerViewMode::Home,
        payload.user_id.to_string(),
    ))
    .await;

    if let Err(message) = slack_state
        .open_session()
        .views_open(&SlackApiViewsOpenRequest {
            trigger_id: payload.trigger_id,
            view,
        })
        .await
    {
        error!("{}", message);
    }

    (StatusCode::OK, to_value("test").unwrap())
}
