use lambda_http::{
    handler,
    lambda_runtime::{self, Context, Error},
    IntoResponse, Request, RequestExt,
};
use receptionist::{handle_slack_event, SlackEventSignatureVerifier, SlackStateWorkaround};
use slack_morphism::prelude::SlackPushEvent;
use tokio::sync::OnceCell;
use tracing::debug;

pub static SLACK_CONFIG: OnceCell<SlackStateWorkaround> = OnceCell::const_new();
pub async fn get_or_init_slack_state() -> &'static SlackStateWorkaround {
    SLACK_CONFIG
        .get_or_init(|| async { SlackStateWorkaround::new_from_env() })
        .await
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    // You can view the logs emitted by your app in Amazon CloudWatch.
    tracing_subscriber::fmt::init();
    debug!("logger has been set up");

    lambda_runtime::run(handler(events_api_lambda)).await?;

    Ok(())
}

async fn events_api_lambda(req: Request, _ctx: Context) -> Result<impl IntoResponse, Error> {
    verify_apig_req_from_slack(&req);

    let push_event_callback = req
        .payload::<SlackPushEvent>()
        .expect("unable to deserialize")
        .expect("no body provided");

    let slack_state = get_or_init_slack_state().await;
    let event_finished = handle_slack_event(slack_state, push_event_callback).await;

    Ok(event_finished.1)
}

pub fn verify_apig_req_from_slack(event: &Request) {
    let signing_secret =
        std::env::var("SLACK_SIGNING_SECRET").expect("No SLACK_SIGNING_SECRET set in env!");

    let headers = event.headers();

    let body_as_string =
        String::from_utf8(event.body().to_vec()).expect("Unable to convert APIG Event to string");

    let timestamp = headers[SlackEventSignatureVerifier::SLACK_SIGNED_TIMESTAMP]
        .to_str()
        .expect("header not a string");

    let signature = headers[SlackEventSignatureVerifier::SLACK_SIGNED_HASH_HEADER]
        .to_str()
        .expect("header not a string");

    SlackEventSignatureVerifier::new(&signing_secret)
        .verify(signature, &body_as_string, timestamp)
        .expect("Verificaction failed, cannnot trust API Gateway Request is from Slack");
}
