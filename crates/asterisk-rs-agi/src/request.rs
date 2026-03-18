use std::collections::HashMap;

use tokio::io::AsyncBufReadExt;

/// parsed AGI request environment sent by asterisk on connection
#[derive(Debug, Clone)]
pub struct AgiRequest {
    /// all agi_* variables as key-value pairs (key without "agi_" prefix)
    variables: HashMap<String, String>,
}

impl AgiRequest {
    /// parse agi environment variables from the initial connection
    ///
    /// reads lines until a blank line is encountered, stripping the `agi_` prefix
    /// from each key
    pub async fn parse_from_reader<R: tokio::io::AsyncBufRead + Unpin>(
        reader: &mut R,
    ) -> crate::error::Result<Self> {
        let mut variables = HashMap::new();
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).await?;

            // eof or blank line terminates the environment block
            if bytes_read == 0 || line.trim().is_empty() {
                break;
            }

            let trimmed = line.trim();
            if let Some((key, value)) = trimmed.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                // strip the agi_ prefix from keys
                let key = key.strip_prefix("agi_").unwrap_or(key);
                variables.insert(key.to_owned(), value.to_owned());
            }
        }

        Ok(Self { variables })
    }

    /// get the value of `agi_network`
    pub fn network(&self) -> Option<&str> {
        self.variables.get("network").map(String::as_str)
    }

    /// get the value of `agi_network_script`
    pub fn network_script(&self) -> Option<&str> {
        self.variables.get("network_script").map(String::as_str)
    }

    /// get the value of `agi_request`
    pub fn request(&self) -> Option<&str> {
        self.variables.get("request").map(String::as_str)
    }

    /// get the value of `agi_channel`
    pub fn channel(&self) -> Option<&str> {
        self.variables.get("channel").map(String::as_str)
    }

    /// get the value of `agi_language`
    pub fn language(&self) -> Option<&str> {
        self.variables.get("language").map(String::as_str)
    }

    /// get the value of `agi_type`
    pub fn channel_type(&self) -> Option<&str> {
        self.variables.get("type").map(String::as_str)
    }

    /// get the value of `agi_uniqueid`
    pub fn unique_id(&self) -> Option<&str> {
        self.variables.get("uniqueid").map(String::as_str)
    }

    /// get the value of `agi_callerid`
    pub fn caller_id(&self) -> Option<&str> {
        self.variables.get("callerid").map(String::as_str)
    }

    /// get the value of `agi_calleridname`
    pub fn caller_id_name(&self) -> Option<&str> {
        self.variables.get("calleridname").map(String::as_str)
    }

    /// get the value of `agi_context`
    pub fn context(&self) -> Option<&str> {
        self.variables.get("context").map(String::as_str)
    }

    /// get the value of `agi_extension`
    pub fn extension(&self) -> Option<&str> {
        self.variables.get("extension").map(String::as_str)
    }

    /// get the value of `agi_priority`
    pub fn priority(&self) -> Option<&str> {
        self.variables.get("priority").map(String::as_str)
    }

    /// generic accessor for any variable by key (without `agi_` prefix)
    pub fn get(&self, key: &str) -> Option<&str> {
        self.variables.get(key).map(String::as_str)
    }
}
