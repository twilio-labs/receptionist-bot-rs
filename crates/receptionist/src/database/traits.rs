use crate::{ListenerEvent, ReceptionistResponse};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait ReceptionistDatabase {
    async fn get_responses_for_listener(
        listener: ListenerEvent,
    ) -> Result<Vec<ReceptionistResponse>>;

    async fn get_response_by_id(response_id: &str) -> Result<ReceptionistResponse>;

    async fn delete_response(rec_response: ReceptionistResponse) -> Result<()>;

    async fn create_response(rec_response: ReceptionistResponse) -> Result<()>;

    async fn update_response(response: ReceptionistResponse) -> Result<()>;

    async fn get_responses_for_collaborator(user_id: &str) -> Result<Vec<ReceptionistResponse>>;
}
