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


#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tokio::io::BufReader;

    async fn parse(input: &str) -> AgiRequest {
        let mut reader = BufReader::new(Cursor::new(input.as_bytes().to_vec()));
        AgiRequest::parse_from_reader(&mut reader)
            .await
            .expect("parse should succeed")
    }

    #[tokio::test]
    async fn parse_standard_agi_request() {
        let input = "agi_network: yes\n\
            agi_network_script: test.agi\n\
            agi_request: agi://127.0.0.1/test\n\
            agi_channel: SIP/200-00000001\n\
            agi_language: en\n\
            agi_type: SIP\n\
            agi_uniqueid: 1234567890.42\n\
            agi_callerid: 2125551234\n\
            agi_calleridname: John Doe\n\
            agi_context: default\n\
            agi_extension: 100\n\
            agi_priority: 1\n\
            \n";
        let req = parse(input).await;
        assert_eq!(req.network(), Some("yes"));
        assert_eq!(req.network_script(), Some("test.agi"));
        assert_eq!(req.request(), Some("agi://127.0.0.1/test"));
        assert_eq!(req.channel(), Some("SIP/200-00000001"));
        assert_eq!(req.language(), Some("en"));
        assert_eq!(req.channel_type(), Some("SIP"));
        assert_eq!(req.unique_id(), Some("1234567890.42"));
        assert_eq!(req.caller_id(), Some("2125551234"));
        assert_eq!(req.caller_id_name(), Some("John Doe"));
        assert_eq!(req.context(), Some("default"));
        assert_eq!(req.extension(), Some("100"));
        assert_eq!(req.priority(), Some("1"));
    }

    #[tokio::test]
    async fn parse_strips_agi_prefix() {
        let req = parse("agi_language: en\n\n").await;
        // stored without prefix
        assert_eq!(req.get("language"), Some("en"));
        // original key with prefix is not stored
        assert_eq!(req.get("agi_language"), None);
    }

    #[tokio::test]
    async fn parse_preserves_non_agi_keys() {
        let req = parse("custom_var: hello\n\n").await;
        assert_eq!(req.get("custom_var"), Some("hello"));
    }

    #[tokio::test]
    async fn parse_empty_input() {
        let req = parse("").await;
        assert_eq!(req.network(), None);
        assert_eq!(req.channel(), None);
    }

    #[tokio::test]
    async fn parse_eof_without_blank_line() {
        // no trailing blank line — parser reads until eof
        let req = parse("agi_language: en\nagi_type: SIP").await;
        assert_eq!(req.language(), Some("en"));
        assert_eq!(req.channel_type(), Some("SIP"));
    }

    #[tokio::test]
    async fn parse_ignores_lines_without_colon() {
        let input = "agi_language: en\ngarbage line\nagi_type: SIP\n\n";
        let req = parse(input).await;
        assert_eq!(req.language(), Some("en"));
        assert_eq!(req.channel_type(), Some("SIP"));
    }

    #[tokio::test]
    async fn parse_value_with_colons() {
        // value contains colons — only split on first
        let req = parse("agi_request: agi://host:4573/script\n\n").await;
        assert_eq!(req.request(), Some("agi://host:4573/script"));
    }

    #[tokio::test]
    async fn parse_whitespace_trimming() {
        let req = parse("  agi_language  :  en  \n\n").await;
        assert_eq!(req.language(), Some("en"));
    }

    #[tokio::test]
    async fn get_returns_none_for_missing_key() {
        let req = parse("agi_language: en\n\n").await;
        assert_eq!(req.get("nonexistent"), None);
    }

    #[tokio::test]
    async fn get_arbitrary_variable() {
        let req = parse("custom_var: world\n\n").await;
        assert_eq!(req.get("custom_var"), Some("world"));
    }

    #[tokio::test]
    async fn all_accessors_return_none_on_empty() {
        let req = parse("\n").await;
        assert_eq!(req.network(), None);
        assert_eq!(req.network_script(), None);
        assert_eq!(req.request(), None);
        assert_eq!(req.channel(), None);
        assert_eq!(req.language(), None);
        assert_eq!(req.channel_type(), None);
        assert_eq!(req.unique_id(), None);
        assert_eq!(req.caller_id(), None);
        assert_eq!(req.caller_id_name(), None);
        assert_eq!(req.context(), None);
        assert_eq!(req.extension(), None);
        assert_eq!(req.priority(), None);
    }
}