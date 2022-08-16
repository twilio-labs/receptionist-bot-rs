use anyhow::{bail, Result};
use axum::http::Uri;
use receptionist::{
    cloudformation::deploy_mock_receptionist_stack, config::ReceptionistAppConfig, create_response,
    delete_response, get_or_init_dynamo_client, get_response_by_id, get_responses_for_collaborator,
    get_responses_for_listener, mock_receptionist_response, wait_for_table, TABLE_NAME,
};

use std::{
    collections::HashMap,
    env,
    process::{Child, Command},
    thread::sleep,
    time::Duration,
};
use testcontainers::{clients::Cli, core::WaitFor, Image, ImageArgs};

struct LocalstackDynamo {
    env_vars: HashMap<String, String>,
}

impl Default for LocalstackDynamo {
    fn default() -> Self {
        let mut env_vars: HashMap<String, String> = HashMap::new();
        env_vars.insert("SERVICES".to_string(), "dynamodb".to_string());
        env_vars.insert("DEFAULT_REGION".to_string(), "us-east-1".to_string());

        Self {
            env_vars: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Default)]
struct LocalstackDynamoImageArgs {}

impl ImageArgs for LocalstackDynamoImageArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        Box::new(Vec::default().into_iter())
    }
}

impl Image for LocalstackDynamo {
    type Args = LocalstackDynamoImageArgs;

    fn name(&self) -> String {
        "localstack/localstack".to_string()
    }

    fn tag(&self) -> String {
        "0.13.0.8".to_string()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::seconds(10)]
    }

    fn expose_ports(&self) -> Vec<u16> {
        vec![4566, 4571]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }

    // fn volumes(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
    //     Box::new(std::iter::empty())
    // }

    // fn entrypoint(&self) -> Option<String> {
    //     None
    // }

    // fn exec_after_start(&self, cs: testcontainers::core::ContainerState) -> Vec<testcontainers::core::ExecCommand> {
    //     Default::default()
    // }
}

pub async fn setup_mock_dynamo_docker() -> Uri {
    let client = Cli::default();
    let _docker_host = match env::var("DOCKER_HOST") {
        Ok(host_string) => {
            dbg!(&host_string);
            match host_string.parse::<Uri>() {
                // is there a way out of this without changing to String?
                Ok(ok) => ok.host().unwrap().to_owned(),
                Err(_) => "0.0.0.0".to_owned(),
            }
        }
        Err(_) => "0.0.0.0".to_owned(),
    };

    let container = client.run(LocalstackDynamo::default());

    container.start();

    let localstack_port = container.get_host_port(4566);
    let override_url = "localhost".to_string() + ":" + &localstack_port.to_string();

    let uri = Uri::builder()
        .scheme("http")
        .authority(override_url)
        .path_and_query("")
        .build()
        .unwrap();

    wait_for_localstack_container(uri.to_string())
        .await
        .unwrap();

    uri
}

async fn wait_for_localstack_container(container_url: String) -> Result<()> {
    let mut request_count = 0;
    let healthcheck_url = container_url + "health";

    loop {
        let client = reqwest::get(&healthcheck_url).await;

        match client {
            Ok(ok) => {
                if ok.status().eq(&reqwest::StatusCode::from_u16(200).unwrap()) {
                    println!("succeeded starting dynamo container");
                    return Ok(());
                }
            }
            Err(e) => {
                if request_count >= 60 {
                    bail!("unable to connect to container {}", e);
                }
                request_count += 1;
                std::thread::sleep(std::time::Duration::from_secs(1))
            }
        }
    }
}

fn start_real_server(aws_endpoint_url: Option<String>) -> Child {
    let mut cmd = Command::new("target/debug/receptionist-bot-rs");

    if let Some(aws_url) = aws_endpoint_url {
        cmd.arg("--aws-endpoint-url").arg(aws_url);
    }

    let child = cmd.spawn().expect("Failed to start receptionist service");
    sleep(Duration::from_secs(10));

    child
}

fn kill_server(mut child_process: Child) {
    dbg!(&child_process.stdout);
    child_process
        .kill()
        .expect("Test APP process did not terminate properly");
}

#[tokio::test]
async fn tester_2_electric_boogaloo() {
    // create container (it will automatically be killed when dropped from memory)
    let client = Cli::default();
    let container = client.run(LocalstackDynamo::default());
    container.start();

    let localstack_port = container.get_host_port(4566);
    let override_url = "localhost".to_string() + ":" + &localstack_port.to_string();

    let uri = Uri::builder()
        .scheme("http")
        .authority(override_url)
        .path_and_query("")
        .build()
        .unwrap();

    // healthcheck that container is running
    wait_for_localstack_container(uri.to_string())
        .await
        .expect("unable to reach container");

    ReceptionistAppConfig::set_mock_env_vars(uri.to_string());

    // println!("big sleep");
    // std::thread::sleep(std::time::Duration::from_secs(2000));
    deploy_mock_receptionist_stack(uri.to_string())
        .await
        .unwrap();

    wait_for_table("table_name", &uri.to_string()).await;

    let mock_1 = mock_receptionist_response();
    let mock_2 = mock_receptionist_response();
    let mock_3 = mock_receptionist_response();

    create_response(mock_1.clone()).await.unwrap();
    create_response(mock_2.clone()).await.unwrap();
    create_response(mock_3.clone()).await.unwrap();

    let query_result = get_responses_for_listener(mock_1.clone().listener)
        .await
        .unwrap();

    assert_eq!(query_result.len(), 3);

    let found_response = get_response_by_id(&mock_1.id).await.unwrap();

    assert_eq!(mock_1.clone(), found_response);

    let _resp = delete_response(mock_1.clone()).await.unwrap();

    let query_result = get_responses_for_listener(mock_1.clone().listener)
        .await
        .unwrap();

    assert_eq!(query_result.len(), 2);

    let client = get_or_init_dynamo_client().await;

    let scan_result = client.scan().table_name(TABLE_NAME).send().await.unwrap();

    dbg!(scan_result);

    let collab = mock_1.collaborators.first().unwrap();
    dbg!(&collab);

    let result = get_responses_for_collaborator(collab).await.unwrap();

    assert_eq!(result.len(), 2);
}
