//! asterisk system operations.

use crate::client::{url_encode, AriClient};
use crate::error::Result;
use serde::Deserialize;

/// asterisk system information
#[derive(Debug, Clone, Deserialize)]
pub struct AsteriskInfo {
    #[serde(default)]
    pub build: Option<serde_json::Value>,
    #[serde(default)]
    pub config: Option<serde_json::Value>,
    #[serde(default)]
    pub status: Option<serde_json::Value>,
    #[serde(default)]
    pub system: Option<serde_json::Value>,
}

/// asterisk ping response
#[derive(Debug, Clone, Deserialize)]
pub struct AsteriskPing {
    pub asterisk_id: String,
    pub ping: String,
    pub timestamp: String,
}

/// loaded asterisk module
#[derive(Debug, Clone, Deserialize)]
pub struct Module {
    pub name: String,
    pub description: String,
    pub use_count: i32,
    pub status: String,
    #[serde(default)]
    pub support_level: Option<String>,
}

/// asterisk log channel
#[derive(Debug, Clone, Deserialize)]
pub struct LogChannel {
    pub channel: String,
    #[serde(rename = "type")]
    pub log_type: String,
    pub status: String,
    pub configuration: String,
}

/// config tuple for dynamic config
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct ConfigTuple {
    pub attribute: String,
    pub value: String,
}

/// variable value response
#[derive(Debug, Clone, Deserialize)]
pub struct Variable {
    pub value: String,
}

/// get asterisk system information
pub async fn info(client: &AriClient, only: Option<&str>) -> Result<AsteriskInfo> {
    let path = match only {
        Some(filter) => format!("/asterisk/info?only={}", url_encode(filter)),
        None => "/asterisk/info".to_string(),
    };
    client.get(&path).await
}

/// ping asterisk
pub async fn ping(client: &AriClient) -> Result<AsteriskPing> {
    client.get("/asterisk/ping").await
}

/// list all loaded modules
pub async fn list_modules(client: &AriClient) -> Result<Vec<Module>> {
    client.get("/asterisk/modules").await
}

/// get details for a specific module
pub async fn get_module(client: &AriClient, module_name: &str) -> Result<Module> {
    client
        .get(&format!("/asterisk/modules/{module_name}"))
        .await
}

/// load a module
pub async fn load_module(client: &AriClient, module_name: &str) -> Result<()> {
    client
        .post_empty(&format!("/asterisk/modules/{module_name}"))
        .await
}

/// unload a module
pub async fn unload_module(client: &AriClient, module_name: &str) -> Result<()> {
    client
        .delete(&format!("/asterisk/modules/{module_name}"))
        .await
}

/// reload a module
pub async fn reload_module(client: &AriClient, module_name: &str) -> Result<()> {
    client
        .put_empty(&format!("/asterisk/modules/{module_name}"))
        .await
}

/// list log channels
pub async fn list_log_channels(client: &AriClient) -> Result<Vec<LogChannel>> {
    client.get("/asterisk/logging").await
}

/// add a log channel
pub async fn add_log_channel(
    client: &AriClient,
    log_channel_name: &str,
    configuration: &str,
) -> Result<()> {
    client
        .post_empty(&format!(
            "/asterisk/logging/{log_channel_name}?configuration={}",
            url_encode(configuration)
        ))
        .await
}

/// remove a log channel
pub async fn remove_log_channel(client: &AriClient, log_channel_name: &str) -> Result<()> {
    client
        .delete(&format!("/asterisk/logging/{log_channel_name}"))
        .await
}

/// rotate a log channel
pub async fn rotate_log_channel(client: &AriClient, log_channel_name: &str) -> Result<()> {
    client
        .put_empty(&format!("/asterisk/logging/{log_channel_name}/rotate"))
        .await
}

/// get a global variable
pub async fn get_variable(client: &AriClient, variable: &str) -> Result<Variable> {
    client
        .get(&format!(
            "/asterisk/variable?variable={}",
            url_encode(variable)
        ))
        .await
}

/// set a global variable
pub async fn set_variable(client: &AriClient, variable: &str, value: &str) -> Result<()> {
    client
        .post_empty(&format!(
            "/asterisk/variable?variable={}&value={}",
            url_encode(variable),
            url_encode(value)
        ))
        .await
}

/// get dynamic configuration object
pub async fn get_config(
    client: &AriClient,
    config_class: &str,
    object_type: &str,
    id: &str,
) -> Result<Vec<ConfigTuple>> {
    client
        .get(&format!(
            "/asterisk/config/dynamic/{config_class}/{object_type}/{id}"
        ))
        .await
}

/// update dynamic configuration object
pub async fn update_config(
    client: &AriClient,
    config_class: &str,
    object_type: &str,
    id: &str,
    fields: &[ConfigTuple],
) -> Result<Vec<ConfigTuple>> {
    client
        .put(
            &format!("/asterisk/config/dynamic/{config_class}/{object_type}/{id}"),
            &fields,
        )
        .await
}

/// delete dynamic configuration object
pub async fn delete_config(
    client: &AriClient,
    config_class: &str,
    object_type: &str,
    id: &str,
) -> Result<()> {
    client
        .delete(&format!(
            "/asterisk/config/dynamic/{config_class}/{object_type}/{id}"
        ))
        .await
}
