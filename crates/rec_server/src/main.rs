#[cfg(all(feature = "tempdb", feature = "dynamodb"))]
compile_error!("cannot enable multiple db features");

/// Starts a Receptionist Webserver to process incoming Slack events, commands, and interactions.
use axum::{
    routing::{get, post},
    AddExtensionLayer, Router,
};
#[cfg(feature = "tempdb")]
use receptionist::get_or_init_mem_db;

use receptionist::{
    axum_handler_handle_slack_commands_api, axum_handler_slack_events_api,
    axum_handler_slack_interactions_api, config::get_or_init_app_config, setup_slack,
    verification::SlackRequestVerifier, ServiceBuilder, SlackEventSignatureVerifier,
};
use std::env;
use tower_http::trace::TraceLayer;
use tracing::info;

#[tokio::main]
async fn main() {
    setup_tracing();
    info!("starting server..");

    // SETUP SHARED BOT RESPONSES CACHE
    let _config = get_or_init_app_config().await.clone();

    #[cfg(feature = "tempdb")]
    get_or_init_mem_db().await;

    // create_response(mock_receptionist_response()).await.unwrap();

    let slack_arc = setup_slack();

    // group slack routes into a separate Router so we can use basepath `/slack` & apply slack auth middleware
    let slack_api_router = Router::new()
        .route("/events", post(axum_handler_slack_events_api))
        .route("/interaction", post(axum_handler_slack_interactions_api))
        .route("/commands", post(axum_handler_handle_slack_commands_api))
        .layer(ServiceBuilder::new().layer_fn(|inner| {
            SlackRequestVerifier {
                inner,
                verifier: SlackEventSignatureVerifier::new(
                    &env::var("SLACK_SIGNING_SECRET")
                        .expect("Provide signing secret env var SLACK_SIGNING_SECRET"),
                ),
            }
        }));

    let app = Router::new()
        .nest("/slack", slack_api_router)
        .route("/", get(|| async { "Hello, World!" }))
        .layer(TraceLayer::new_for_http())
        .layer(AddExtensionLayer::new(slack_arc));

    // .layer(AddExtensionLayer::new(app_responses_cache));

    let host_address = env::var("HOST_ADDRESS").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    tracing::debug!("listening on {}", &host_address);
    // run it with hyper
    axum::Server::bind(&host_address.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn setup_tracing() {
    // Set the RUST_LOG, if it hasn't been explicitly defined
    if std::env::var_os("RUST_LOG").is_none() {
        // std::env::set_var("RUST_LOG", "receptionist_bot_rs=debug,tower_http=debug")
        std::env::set_var(
            "RUST_LOG",
            "receptionist_bot_rs=trace,tower_http=trace,receptionist=trace",
        )
    }
    tracing_subscriber::fmt::init();
}
