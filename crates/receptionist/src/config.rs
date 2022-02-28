use crate::PagerDuty;
use arguably::ArgParser;
use aws_sdk_dynamodb::Credentials;
use dotenv::dotenv;
use tokio::sync::OnceCell;

pub static APP_CONFIG: OnceCell<ReceptionistAppConfig> = OnceCell::const_new();
pub async fn get_or_init_app_config() -> &'static ReceptionistAppConfig {
    APP_CONFIG
        .get_or_init(|| async { ReceptionistAppConfig::new() })
        .await
}

const CLI_OPTION_AWS_URL: &str = "aws-endpoint-url";
const ENV_FLAG_AWS_ENDPOINT_URL: &str = "AWS_ENDPOINT_URL";
const ENV_FLAG_AWS_FAKE_CREDS: &str = "AWS_FAKE_CREDS";
pub const ENV_OPTION_PD_KEY: &str = "PAGERDUTY_TOKEN";
pub const ENV_OPTION_PD_BASE_URL: &str = "PAGERDUTY_BASE_URL";

#[derive(Clone)]
/// can load a .env file to the environment and parse cli args to build the app config
pub struct ReceptionistAppConfig {
    pub aws_override_url: Option<String>,
    pub aws_fake_creds: Option<Credentials>,
    /// if no pagerduty configuration, remove pagerduty actions from Response Creator modal
    pub pagerduty_config: Option<PagerDuty>,
}

impl ReceptionistAppConfig {
    /// load a .env file to the environment and parse cli args to build the app config
    /// Supported .env strings:
    ///
    /// AWS_ENDPOINT_URL
    /// PAGERDUTY_TOKEN
    /// PAGERDUTY_BASE_URL
    ///
    /// Supported .env boolean flags:
    ///
    /// AWS_FAKE_CREDS
    ///
    pub fn new() -> Self {
        dotenv().ok();

        let mut parser = ArgParser::new()
            .option(CLI_OPTION_AWS_URL, "")
            .flag("fake")
            .helptext(format!(
                "Usage: An alternate aws url can be provided via the cli arg `--{CLI_OPTION_AWS_URL}` or by setting \
the environment variable `{ENV_FLAG_AWS_ENDPOINT_URL}`\n Fake AWS Creds can automatticaly be applied if either the flag --fake is present or env var {ENV_FLAG_AWS_FAKE_CREDS} is true "
            ));

        if let Err(e) = parser.parse() {
            e.exit();
        }

        let aws_override_url = if parser.found(CLI_OPTION_AWS_URL) {
            Some(parser.value(CLI_OPTION_AWS_URL))
        } else {
            std::env::var(ENV_FLAG_AWS_ENDPOINT_URL).ok()
        };

        let aws_fake_creds =
            if parser.found("fake") || std::env::var(ENV_FLAG_AWS_FAKE_CREDS).is_ok() {
                Some(Credentials::new("test", "test", None, None, "test"))
            } else {
                None
            };

        let pagerduty_config = std::env::var(ENV_OPTION_PD_KEY).map_or(None, |pd_key| {
            Some(PagerDuty::new(
                pd_key,
                std::env::var(ENV_OPTION_PD_BASE_URL).ok(),
            ))
        });

        Self {
            aws_override_url,
            aws_fake_creds,
            pagerduty_config,
        }
    }

    pub fn set_mock_env_vars(aws_url: String) {
        std::env::set_var(ENV_FLAG_AWS_ENDPOINT_URL, aws_url);
        std::env::set_var(ENV_FLAG_AWS_FAKE_CREDS, "TRUE");
    }
}

impl Default for ReceptionistAppConfig {
    fn default() -> Self {
        Self::new()
    }
}
