//! endpoint query operations (read-only).

use crate::client::{url_encode, AriClient};
use crate::error::Result;

/// ari endpoint representation
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Endpoint {
    pub technology: String,
    pub resource: String,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub channel_ids: Vec<String>,
}

/// list all endpoints
pub async fn list(client: &AriClient) -> Result<Vec<Endpoint>> {
    client.get("/endpoints").await
}

/// list endpoints for a specific technology
pub async fn list_by_tech(client: &AriClient, tech: &str) -> Result<Vec<Endpoint>> {
    client
        .get(&format!("/endpoints/{}", url_encode(tech)))
        .await
}

/// get a specific endpoint
pub async fn get(client: &AriClient, tech: &str, resource: &str) -> Result<Endpoint> {
    client
        .get(&format!(
            "/endpoints/{}/{}",
            url_encode(tech),
            url_encode(resource)
        ))
        .await
}

/// send a message to some technology uri or endpoint
pub async fn send_message(client: &AriClient, to: &str, from: &str, body: &str) -> Result<()> {
    client
        .put_empty(&format!(
            "/endpoints/sendMessage?to={}&from={}&body={}",
            url_encode(to),
            url_encode(from),
            url_encode(body)
        ))
        .await
}

/// refer an endpoint or technology uri to some technology uri or endpoint
pub async fn refer(client: &AriClient, to: &str, from: &str, refer_to: &str) -> Result<()> {
    client
        .post_empty(&format!(
            "/endpoints/refer?to={}&from={}&refer_to={}",
            url_encode(to),
            url_encode(from),
            url_encode(refer_to)
        ))
        .await
}

/// send a message to an endpoint
pub async fn send_message_to_endpoint(
    client: &AriClient,
    tech: &str,
    resource: &str,
    from: &str,
    body: &str,
) -> Result<()> {
    client
        .put_empty(&format!(
            "/endpoints/{}/{}/sendMessage?from={}&body={}",
            url_encode(tech),
            url_encode(resource),
            url_encode(from),
            url_encode(body)
        ))
        .await
}

/// refer an endpoint to some technology uri or endpoint
pub async fn refer_to_endpoint(
    client: &AriClient,
    tech: &str,
    resource: &str,
    from: &str,
    refer_to: &str,
) -> Result<()> {
    client
        .post_empty(&format!(
            "/endpoints/{}/{}/refer?from={}&refer_to={}",
            url_encode(tech),
            url_encode(resource),
            url_encode(from),
            url_encode(refer_to)
        ))
        .await
}
