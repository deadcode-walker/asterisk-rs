//! mailbox operations.

use crate::client::{url_encode, AriClient};
use crate::error::Result;

/// ari mailbox representation
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Mailbox {
    pub name: String,
    pub old_messages: u32,
    pub new_messages: u32,
}

/// list all mailboxes
pub async fn list(client: &AriClient) -> Result<Vec<Mailbox>> {
    client.get("/mailboxes").await
}

/// get a specific mailbox
pub async fn get(client: &AriClient, name: &str) -> Result<Mailbox> {
    client
        .get(&format!("/mailboxes/{}", url_encode(name)))
        .await
}

/// update a mailbox message count
///
/// note: asterisk ari spec uses PUT for this endpoint, but asterisk
/// also accepts POST for compatibility
pub async fn update(
    client: &AriClient,
    name: &str,
    old_messages: u32,
    new_messages: u32,
) -> Result<()> {
    client
        .post_empty(&format!(
            "/mailboxes/{}?oldMessages={old_messages}&newMessages={new_messages}",
            url_encode(name)
        ))
        .await
}

/// delete a mailbox
pub async fn delete(client: &AriClient, name: &str) -> Result<()> {
    client
        .delete(&format!("/mailboxes/{}", url_encode(name)))
        .await
}
