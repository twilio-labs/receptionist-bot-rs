use crate::config::get_or_init_app_config;
use crate::ReceptionistError;
use crate::ReceptionistListener;
use crate::ReceptionistResponse;

// use anyhow::{anyhow, bail, Result};
use aws_sdk_dynamodb::model::{
    AttributeValue, DeleteRequest, KeysAndAttributes, PutRequest, WriteRequest,
};
use aws_sdk_dynamodb::output::BatchWriteItemOutput;
use aws_sdk_dynamodb::{Client, Config, Endpoint, Region};
use aws_types::Credentials;
use serde::{Deserialize, Serialize};
use serde_dynamo::aws_sdk_dynamodb_0_4::{from_item, from_items, to_attribute_value, to_item};
use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;
use std::time::Duration;
use tokio::sync::OnceCell;
// Starter examples: https://github.com/awslabs/aws-sdk-rust/tree/main/examples/dynamodb/src/bin

pub const TABLE_NAME: &str = "receptionist_bot";
pub const INDEX_NAME: &str = "InvertedIndex";

pub static DYNAMO_CLIENT: OnceCell<aws_sdk_dynamodb::Client> = OnceCell::const_new();
pub async fn get_or_init_dynamo_client() -> &'static aws_sdk_dynamodb::Client {
    DYNAMO_CLIENT
        .get_or_init(|| async {
            let app_config = get_or_init_app_config().await;
            setup_dynamo_client(
                None,
                app_config.aws_override_url.clone(),
                app_config.aws_fake_creds.clone(),
            )
            .await
        })
        .await
}

pub async fn setup_dynamo_client(
    override_region: Option<String>,
    override_url: Option<String>,
    override_credentials: Option<Credentials>,
) -> Client {
    if override_region.is_some() || override_url.is_some() || override_credentials.is_some() {
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

        Client::from_conf(new_config.build())
    } else {
        let shared_config = aws_config::load_from_env().await;
        Client::new(&shared_config)
    }
}

pub async fn create_response(
    rec_response: ReceptionistResponse,
) -> Result<BatchWriteItemOutput, ReceptionistError> {
    let client = get_or_init_dynamo_client().await;

    let table_items_before_formatting = convert_response_to_table_items(rec_response)?;

    // using a map iterator is breaking here when ? operator used :shrug:
    let mut all_items: Vec<HashMap<String, AttributeValue>> = Vec::new();
    for item in table_items_before_formatting {
        all_items.push(to_item(item)?)
    }

    let output = client
        .batch_write_item()
        .request_items(
            TABLE_NAME.to_string(),
            all_items
                .into_iter()
                .map(|item| {
                    WriteRequest::builder()
                        .put_request(PutRequest::builder().set_item(Some(item)).build())
                        .build()
                })
                .collect::<Vec<WriteRequest>>(),
        )
        .send()
        .await?;

    Ok(output)
}

pub async fn get_responses_for_listener(
    listener: ReceptionistListener,
) -> Result<Vec<ReceptionistResponse>, ReceptionistError> {
    let client = get_or_init_dynamo_client().await;

    let result = client
        .query()
        .table_name(TABLE_NAME)
        .key_condition_expression("pk = :listener_str")
        .expression_attribute_values(
            ":listener_str",
            to_attribute_value(ListenerPKey::from(listener).to_string())?,
        )
        .send()
        .await?;

    if result.count() == 0 {
        return Ok(Vec::new());
    }

    Ok(from_items(result.items().unwrap().to_owned())?)
}

pub async fn get_response_by_id(
    response_id: &str,
) -> Result<ReceptionistResponse, ReceptionistError> {
    let client = get_or_init_dynamo_client().await;

    let result = client
        .query()
        .table_name(TABLE_NAME)
        .index_name(INDEX_NAME)
        .key_condition_expression("sk = :response_id")
        .expression_attribute_values(":response_id", to_attribute_value(response_id.to_string())?)
        .send()
        .await?;

    let count = result.count();
    let items = result.items();

    if count == 0 {
        Err(ReceptionistError::DatabaseError("Item not found".into()))?
    } else {
        let mut all_responses: Vec<ReceptionistResponse> = Vec::new();
        for item in items.unwrap().to_owned() {
            let as_item_struct: ReceptionistTableItem = from_item(item)?;
            match as_item_struct {
                ReceptionistTableItem::Response {
                    receptionist_response,
                    ..
                } => all_responses.push(receptionist_response),
                ReceptionistTableItem::Collaborator { .. } => (),
            }
        }

        if all_responses.len() != 1 {
            Err(ReceptionistError::DatabaseError(format!(
                "More than 1 response found for this ID: {}",
                response_id
            )))?
        }

        return Ok(all_responses.first().unwrap().to_owned());
    }
}

pub async fn delete_response(
    rec_response: ReceptionistResponse,
) -> Result<BatchWriteItemOutput, ReceptionistError> {
    let client = get_or_init_dynamo_client().await;

    let table_items_before_formatting = convert_response_to_table_items(rec_response)?;

    let output = client
        .batch_write_item()
        .request_items(
            TABLE_NAME.to_string(),
            table_items_before_formatting
                .into_iter()
                .map(|item| {
                    let (pk, sk) = item.get_pk_sk_strings();
                    WriteRequest::builder()
                        // .put_request(PutRequest::builder().set_item(Some(item)).build())
                        .delete_request(
                            DeleteRequest::builder()
                                .key("pk", to_attribute_value(pk).unwrap())
                                .key("sk", to_attribute_value(sk).unwrap())
                                .build(),
                        )
                        .build()
                })
                .collect::<Vec<WriteRequest>>(),
        )
        .send()
        .await?;

    Ok(output)
}

pub async fn update_response(
    response: ReceptionistResponse,
) -> Result<BatchWriteItemOutput, ReceptionistError> {
    create_response(response).await
}

async fn get_collaborator_items(
    user_id: &str,
) -> Result<Vec<ReceptionistTableItem>, ReceptionistError> {
    let client = get_or_init_dynamo_client().await;

    let result = client
        .query()
        .table_name(TABLE_NAME)
        .key_condition_expression("pk = :collaborator_id")
        .expression_attribute_values(":collaborator_id", to_attribute_value(user_id.to_string())?)
        .send()
        .await?;

    if result.count() == 0 {
        return Ok(Vec::new());
    }

    Ok(from_items::<ReceptionistTableItem>(
        result.items().unwrap().to_owned(),
    )?)
}

pub async fn get_responses_for_collaborator(
    user_id: &str,
) -> Result<Vec<ReceptionistResponse>, ReceptionistError> {
    let collaborator_items = get_collaborator_items(user_id).await?;

    if collaborator_items.is_empty() {
        return Ok(Vec::new());
    }

    let mut keys_builder = KeysAndAttributes::builder();

    for item in collaborator_items {
        match item {
            ReceptionistTableItem::Collaborator {
                sk, listener_pk, ..
            } => {
                keys_builder = keys_builder.keys(
                    [
                        ("pk".to_owned(), to_attribute_value(listener_pk).unwrap()),
                        ("sk".to_owned(), to_attribute_value(sk).unwrap()),
                    ]
                    .into_iter()
                    .collect::<HashMap<String, AttributeValue>>(),
                );
            }
            _ => panic!("only collaborator items should be in this query"),
        }
    }

    let client = get_or_init_dynamo_client().await;
    let result = client
        .batch_get_item()
        .request_items(TABLE_NAME, keys_builder.build())
        .send()
        .await?;

    Ok(from_items(
        result
            .responses
            .unwrap()
            .get(TABLE_NAME)
            .unwrap()
            .to_owned(),
    )?)

    // alternate method of writing the above without mutating array:
    // Ok(table_items
    //     .into_iter()
    //     .filter(|item| matches!(item, &ReceptionistTableItem::Response { .. }))
    //     .map(|item| match item {
    //         ReceptionistTableItem::Response {
    //             receptionist_response,
    //             ..
    //         } => receptionist_response,
    //         _ => panic!("already matched, this shouldn't happen"),
    //     })
    //     .collect())
}

pub async fn build_mock_client(override_url: &str) -> Client {
    let region = Region::new("us-east-1");
    let credentials = Credentials::new("test", "test", None, None, "yaboy");
    let mut new_config = Config::builder();

    let url_override = Endpoint::immutable(override_url.parse().expect("Failed to parse URI"));
    new_config = new_config.endpoint_resolver(url_override);

    new_config = new_config.region(region).credentials_provider(credentials);

    Client::from_conf(new_config.build())
}

pub async fn wait_for_table(_table_name: &str, _override_url: &str) {
    let client = get_or_init_dynamo_client().await;

    let mut count = 0;
    while count < 240 {
        println!("checking for table");
        let res = client.list_tables().send().await.unwrap();
        dbg!(&res);

        if res.table_names.unwrap().len() == 1 {
            println!("FOUND");
            return;
        } else {
            count += 1;
            std::thread::sleep(Duration::from_secs_f32(0.25));
        }
    }
    panic!("Table not found");
}

fn convert_response_to_table_items(
    rec_response: ReceptionistResponse,
) -> Result<Vec<ReceptionistTableItem>, ReceptionistError> {
    let mut all_items = vec![];

    let response_pkey: ListenerPKey = rec_response.listener.clone().into();
    all_items.push(rec_response.clone().into());

    for collaborator in rec_response.collaborators {
        all_items.push(ReceptionistTableItem::Collaborator {
            pk: collaborator,
            sk: rec_response.id.clone(),
            listener_pk: response_pkey.clone(),
        })
    }

    Ok(all_items)
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "item_type")]
enum ReceptionistTableItem {
    Response {
        /// Bot's Response Listener
        pk: ListenerPKey,
        /// Bot's Response ID: ABC123456
        sk: String,
        #[serde(flatten)]
        receptionist_response: ReceptionistResponse,
    },
    Collaborator {
        /// SlackUserID
        pk: String,
        /// Bot's Response ID
        sk: String,
        listener_pk: ListenerPKey,
    },
}

impl ReceptionistTableItem {
    fn get_pk_sk_strings(&self) -> (String, String) {
        match self {
            ReceptionistTableItem::Response { pk, sk, .. } => (pk.to_string(), sk.to_string()),
            ReceptionistTableItem::Collaborator { pk, sk, .. } => (pk.to_string(), sk.to_string()),
        }
    }
}

impl From<ReceptionistResponse> for ReceptionistTableItem {
    fn from(rec_response: ReceptionistResponse) -> Self {
        Self::Response {
            pk: rec_response.listener.clone().into(),
            sk: rec_response.id.clone(),
            receptionist_response: rec_response,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
/// String Representation of a Receptionist Listener: `slack-channel/C23456`
struct ListenerPKey(String);

impl Display for ListenerPKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<ReceptionistListener> for ListenerPKey {
    fn from(listener: ReceptionistListener) -> Self {
        let pk = match listener.clone() {
            ReceptionistListener::SlackChannel { channel_id } => {
                format!("{}/{}", listener, channel_id)
            }
        };

        Self(pk)
    }
}

impl TryInto<ReceptionistListener> for ListenerPKey {
    type Error = ReceptionistError;

    fn try_into(self) -> Result<ReceptionistListener, ReceptionistError> {
        let (listener_type, value) = self.0.split_once("/").ok_or_else(|| {
            ReceptionistError::DatabaseError(
                "Unable to find PKey delimiter to convert response to dynamodb type".into(),
            )
        })?;

        let listener = ReceptionistListener::from_str(listener_type)?;

        match listener {
            ReceptionistListener::SlackChannel { .. } => Ok(ReceptionistListener::SlackChannel {
                channel_id: value.into(),
            }),
        }
    }
}

#[cfg(test)]
mod test {

    use super::convert_response_to_table_items;
    use crate::mock_receptionist_response;
    use anyhow::Result;

    #[test]
    fn test_build_table_items() -> Result<()> {
        let test_resp = mock_receptionist_response();

        let _as_items = convert_response_to_table_items(test_resp).unwrap();

        Ok(())
    }
}
