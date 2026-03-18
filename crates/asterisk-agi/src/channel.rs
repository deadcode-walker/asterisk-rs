use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

use crate::command;
use crate::error::{AgiError, Result};
use crate::response::AgiResponse;

/// high-level interface for sending AGI commands over a connection
pub struct AgiChannel {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
    hung_up: bool,
}

impl AgiChannel {
    /// create a new AGI channel from split TCP stream halves
    pub fn new(reader: BufReader<OwnedReadHalf>, writer: OwnedWriteHalf) -> Self {
        Self {
            reader,
            writer,
            hung_up: false,
        }
    }

    /// send a raw command string and parse the response
    ///
    /// the command should already be formatted with a trailing newline.
    /// checks the hung_up flag before sending to avoid writing to a dead channel.
    pub async fn send_command(&mut self, command: &str) -> Result<AgiResponse> {
        if self.hung_up {
            return Err(AgiError::ChannelHungUp);
        }

        self.writer.write_all(command.as_bytes()).await?;
        self.writer.flush().await?;

        let mut line = String::new();
        let bytes_read = self.reader.read_line(&mut line).await?;

        if bytes_read == 0 {
            self.hung_up = true;
            return Err(AgiError::ChannelHungUp);
        }

        let response = AgiResponse::parse(&line)?;

        // 511 means the channel is dead
        if response.code == 511 {
            self.hung_up = true;
            return Err(AgiError::ChannelHungUp);
        }

        Ok(response)
    }

    /// answer the channel
    pub async fn answer(&mut self) -> Result<AgiResponse> {
        let cmd = command::format_command(command::ANSWER, &[]);
        self.send_command(&cmd).await
    }

    /// hang up the channel, optionally specifying which channel to hang up
    pub async fn hangup(&mut self, channel: Option<&str>) -> Result<AgiResponse> {
        let cmd = match channel {
            Some(ch) => command::format_command(command::HANGUP, &[ch]),
            None => command::format_command(command::HANGUP, &[]),
        };
        self.send_command(&cmd).await
    }

    /// stream a sound file, allowing the caller to interrupt with escape digits
    pub async fn stream_file(
        &mut self,
        filename: &str,
        escape_digits: &str,
    ) -> Result<AgiResponse> {
        let cmd = command::format_command(command::STREAM_FILE, &[filename, escape_digits]);
        self.send_command(&cmd).await
    }

    /// play a prompt and collect DTMF digits
    pub async fn get_data(
        &mut self,
        filename: &str,
        timeout_ms: u64,
        max_digits: u32,
    ) -> Result<AgiResponse> {
        let timeout = timeout_ms.to_string();
        let digits = max_digits.to_string();
        let cmd = command::format_command(command::GET_DATA, &[filename, &timeout, &digits]);
        self.send_command(&cmd).await
    }

    /// say a digit string with escape digits
    pub async fn say_digits(&mut self, digits: &str, escape_digits: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SAY_DIGITS, &[digits, escape_digits]);
        self.send_command(&cmd).await
    }

    /// say a number with escape digits
    pub async fn say_number(&mut self, number: i64, escape_digits: &str) -> Result<AgiResponse> {
        let num = number.to_string();
        let cmd = command::format_command(command::SAY_NUMBER, &[&num, escape_digits]);
        self.send_command(&cmd).await
    }

    /// set a channel variable
    pub async fn set_variable(&mut self, name: &str, value: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SET_VARIABLE, &[name, value]);
        self.send_command(&cmd).await
    }

    /// get a channel variable
    pub async fn get_variable(&mut self, name: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::GET_VARIABLE, &[name]);
        self.send_command(&cmd).await
    }

    /// execute an asterisk application
    pub async fn exec(&mut self, application: &str, args: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::EXEC, &[application, args]);
        self.send_command(&cmd).await
    }

    /// wait for a DTMF digit, -1 for infinite timeout
    pub async fn wait_for_digit(&mut self, timeout_ms: i64) -> Result<AgiResponse> {
        let timeout = timeout_ms.to_string();
        let cmd = command::format_command(command::WAIT_FOR_DIGIT, &[&timeout]);
        self.send_command(&cmd).await
    }

    /// get the status of a channel
    pub async fn channel_status(&mut self, channel: Option<&str>) -> Result<AgiResponse> {
        let cmd = match channel {
            Some(ch) => command::format_command(command::CHANNEL_STATUS, &[ch]),
            None => command::format_command(command::CHANNEL_STATUS, &[]),
        };
        self.send_command(&cmd).await
    }

    /// send a verbose message to the asterisk console
    pub async fn verbose(&mut self, message: &str, level: u8) -> Result<AgiResponse> {
        let lvl = level.to_string();
        let cmd = command::format_command(command::VERBOSE, &[message, &lvl]);
        self.send_command(&cmd).await
    }
}
