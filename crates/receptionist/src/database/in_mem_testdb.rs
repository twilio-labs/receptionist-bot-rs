use crate::write_serde_struct_to_file;
use crate::ReceptionistListener;
use crate::ReceptionistResponse;
use anyhow::{bail, Result};
use std::collections::HashMap;
use tokio::sync::OnceCell;
use tokio::sync::RwLock;
use tokio::sync::RwLockReadGuard;

pub static IN_MEM_DB: OnceCell<tokio::sync::RwLock<HashMap<String, ReceptionistResponse>>> =
    OnceCell::const_new();
pub async fn get_or_init_mem_db() -> &'static RwLock<HashMap<String, ReceptionistResponse>> {
    IN_MEM_DB
        .get_or_init(|| async {
            let hash_map: HashMap<String, ReceptionistResponse> = HashMap::new();
            RwLock::new(hash_map)
        })
        .await
}

pub fn save_db_to_json(temp_db: RwLockReadGuard<HashMap<String, ReceptionistResponse>>) {
    let all_responses_as_vec: Vec<&ReceptionistResponse> =
        temp_db.iter().map(|(_k, v)| v).collect();

    write_serde_struct_to_file("all_responses.json", all_responses_as_vec)
}

pub async fn create_response(rec_response: ReceptionistResponse) -> Result<()> {
    let db_lock = get_or_init_mem_db().await;

    let mut all_responses = db_lock.write().await;

    if all_responses.contains_key(&rec_response.id) {
        bail!("ID already exists: {}", rec_response.id)
    } else {
        all_responses.insert(rec_response.id.to_owned(), rec_response);
    }

    // let all_responses_read_only = all_responses.downgrade();
    // save_db_to_json(all_responses_read_only);
    Ok(())
}

pub async fn get_responses_for_listener(
    listener: ReceptionistListener,
) -> Result<Vec<ReceptionistResponse>> {
    let db_key = match listener {
        ReceptionistListener::SlackChannel { channel_id } => channel_id,
    };

    let db_lock = get_or_init_mem_db().await;

    let all_responses = db_lock.read().await;

    Ok(all_responses
        .values()
        .filter(|response| response.listener.matches_slack_channel_id(&db_key))
        .map(|r| r.to_owned())
        .collect())
}

pub async fn get_response_by_id(response_id: &str) -> Result<ReceptionistResponse> {
    let db_lock = get_or_init_mem_db().await;

    let all_responses = db_lock.read().await;

    match all_responses.get(response_id) {
        Some(response) => Ok(response.to_owned()),
        None => bail!("no response found for that id"),
    }
}

pub async fn update_response(response: ReceptionistResponse) -> Result<()> {
    let db_lock = get_or_init_mem_db().await;

    let mut all_responses = db_lock.write().await;

    all_responses.insert(response.id.to_owned(), response);

    Ok(())
}

pub async fn delete_response(response: ReceptionistResponse) -> Result<()> {
    let db_lock = get_or_init_mem_db().await;

    let mut all_responses = db_lock.write().await;

    all_responses.remove(&response.id);

    Ok(())
}

pub async fn get_responses_for_collaborator(user_id: &str) -> Result<Vec<ReceptionistResponse>> {
    let db_lock = get_or_init_mem_db().await;

    let all_responses = db_lock.read().await;

    Ok(all_responses
        .values()
        .filter(|response| response.collaborators.contains(&user_id.to_owned()))
        .map(|r| r.to_owned())
        .collect())
}
