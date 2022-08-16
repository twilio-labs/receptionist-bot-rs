// Copyright (c) 2022 Twilio Inc.

#[cfg(all(feature = "tempdb", feature = "dynamodb"))]
compile_error!("cannot enable multiple db features");

#[macro_use]
extern crate macro_rules_attribute;

derive_alias! {
    #[derive(EnumUtils!)] = #[derive(strum::Display, strum::EnumDiscriminants, strum::EnumIter, Debug, PartialEq)];
}

pub mod config;
mod database;
mod manager_ui;
mod pagerduty;
mod response;
mod slack;
mod utils;

pub use database::*;
pub use manager_ui::*;
pub use pagerduty::client::PagerDuty;
pub use response::*;
pub use response::{mock_receptionist_response, ReceptionistResponse as Receptionistresponse};
pub use slack::*;
pub use tower::ServiceBuilder;
pub use utils::*;
