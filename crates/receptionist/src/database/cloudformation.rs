use anyhow::{anyhow, Result};
use aws_sdk_cloudformation::{
    output::CreateStackOutput, Client, Config, Credentials, Endpoint, Region,
};

pub fn setup_cf_client(
    override_region: Option<String>,
    override_url: Option<String>,
    override_credentials: Option<Credentials>,
) -> Client {
    // let mut shared_config = aws_config::load_from_env().await;
    let mut new_config = Config::builder();

    if let Some(creds) = override_credentials {
        new_config = new_config.credentials_provider(creds);
    }

    new_config = new_config.region(Region::new(
        override_region.unwrap_or_else(|| "us-east-1".to_string()),
    ));

    if let Some(url_override_string) = override_url {
        let url_override =
            Endpoint::immutable(url_override_string.parse().expect("Failed to parse URI"));
        new_config = new_config.endpoint_resolver(url_override);
    };
    // let credentials = Credentials::new("test", "test", None, None, "yaboy");

    Client::from_conf(new_config.build())
}

pub async fn deploy_receptionist_stack(cf_client: Client) -> Result<CreateStackOutput> {
    let template_location = "../receptionist/src/database/dynamo_cf_template.json";
    let template_body = std::fs::read_to_string(template_location).expect("File not found");
    let create_stack_result = cf_client
        .create_stack()
        .set_stack_name(Some("receptionist-bot-supporting-infra".into()))
        .set_template_body(Some(template_body))
        .send()
        .await;

    create_stack_result.map_err(|e| anyhow!("Unable to create stack: {}", e))
}

pub async fn deploy_mock_receptionist_stack(local_url: String) -> Result<CreateStackOutput> {
    let fake_creds = Some(Credentials::new("test", "test", None, None, "test"));
    let client = setup_cf_client(None, Some(local_url), fake_creds);

    let res = deploy_receptionist_stack(client.clone()).await;

    // dbg!(client.list_stacks().send().await?);

    res
}
