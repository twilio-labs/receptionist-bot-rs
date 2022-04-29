use aws_sdk_dynamodb::error::{BatchGetItemError, BatchWriteItemError, QueryError};
use serde::Serialize;

use serde_json::to_writer_pretty;
use std::{fs::File, path::Path};
use thiserror::Error;

/// used during development to capture/inspect for mocking
pub fn write_serde_struct_to_file<P: AsRef<Path>>(path: P, obj: impl Serialize) {
    to_writer_pretty(&File::create(path).expect("unable to create file"), &obj)
        .expect("unable to write to file")
}

#[derive(Error, Debug)]
pub enum ReceptionistError {
    // #[error("data store disconnected")]
    // Disconnect(#[from] io::Error),
    // #[error("the data for key `{0}` is not available")]
    // Redaction(String),
    // #[error("invalid header (expected {expected:?}, found {found:?})")]
    // InvalidHeader { expected: String, found: String },
    // #[error("unknown data store error")]
    // Unknown,
    #[error("Failed during interaction with database: `{0}`")]
    DatabaseError(String),
    #[error("Failed during interaction with Slack Api")]
    SlackApiError,
    #[error("Failed during interaction with Pagerduty Api")]
    PagerdutyApiError,
    #[error("Failed to do configured Response Action")]
    ActionError,
    #[error("Failed to check configured Response Condition")]
    ConditionError,
    #[error("Failed to build an Enum variant from a string")]
    EnumFromStringError(#[from] strum::ParseError),
    #[cfg(feature = "dynamodb")]
    #[error("Failed to convert to or from dynamo item to receptionist bot type")]
    DynamoDBSerializationError(#[from] serde_dynamo::Error),
    #[cfg(feature = "dynamodb")]
    #[error("Item not found in dynamo table")]
    DynamoDBItemNotFoundError(#[from] aws_sdk_dynamodb::SdkError<BatchGetItemError>),
    #[cfg(feature = "dynamodb")]
    #[error("Query failed in dynamo table")]
    DynamoDBQueryError(#[from] aws_sdk_dynamodb::SdkError<QueryError>),
    #[cfg(feature = "dynamodb")]
    #[error("Query failed in dynamo table")]
    DynamoDBWriteError(#[from] aws_sdk_dynamodb::SdkError<BatchWriteItemError>),
}
