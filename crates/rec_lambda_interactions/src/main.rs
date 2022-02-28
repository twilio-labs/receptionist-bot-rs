use lambda_http::{
    handler,
    lambda_runtime::{self, Context, Error},
    Body, IntoResponse, Request, RequestExt, Response,
};
use receptionist::{
    handle_slack_interaction, SlackEventSignatureVerifier, SlackInteractionWrapper,
    SlackStateWorkaround,
};
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

    lambda_runtime::run(handler(interactions_api_lambda)).await?;

    Ok(())
}

async fn interactions_api_lambda(req: Request, _ctx: Context) -> Result<impl IntoResponse, Error> {
    verify_apig_req_from_slack(&req);

    let interaction_event_wrapper = req
        .payload::<SlackInteractionWrapper>()
        .expect("unable to deserialize")
        .expect("no body provided");

    let slack_state = get_or_init_slack_state().await;
    let (status, value) = handle_slack_interaction(slack_state, interaction_event_wrapper).await;

    if value.is_string() && value.as_str().unwrap().is_empty() {
        // need to send the NO_CONTENT status code in order to close the modal (*shake fist*).
        Ok(Response::builder()
            .status(status)
            .body(Body::Empty)
            .unwrap())
    } else {
        // modal validation failed, display the error in the modal
        Ok(value.into_response())
    }
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
