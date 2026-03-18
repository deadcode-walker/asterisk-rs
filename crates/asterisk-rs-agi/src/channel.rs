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

    /// set the caller id for the current channel
    pub async fn set_callerid(&mut self, callerid: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SET_CALLERID, &[callerid]);
        self.send_command(&cmd).await
    }

    /// get a value from the asterisk database
    pub async fn database_get(&mut self, family: &str, key: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::DATABASE_GET, &[family, key]);
        self.send_command(&cmd).await
    }

    /// set a value in the asterisk database
    pub async fn database_put(
        &mut self,
        family: &str,
        key: &str,
        value: &str,
    ) -> Result<AgiResponse> {
        let cmd = command::format_command(command::DATABASE_PUT, &[family, key, value]);
        self.send_command(&cmd).await
    }

    /// delete a key from the asterisk database
    pub async fn database_del(&mut self, family: &str, key: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::DATABASE_DEL, &[family, key]);
        self.send_command(&cmd).await
    }

    /// delete a family or key tree from the asterisk database
    pub async fn database_deltree(
        &mut self,
        family: &str,
        key: Option<&str>,
    ) -> Result<AgiResponse> {
        let cmd = match key {
            Some(k) => command::format_command(command::DATABASE_DELTREE, &[family, k]),
            None => command::format_command(command::DATABASE_DELTREE, &[family]),
        };
        self.send_command(&cmd).await
    }

    /// stream file with ability to control (pause, rewind, fast forward)
    pub async fn control_stream_file(
        &mut self,
        filename: &str,
        escape_digits: &str,
        skipms: Option<i64>,
        ff_char: Option<&str>,
        rew_char: Option<&str>,
        pause_char: Option<&str>,
    ) -> Result<AgiResponse> {
        let skip = skipms.unwrap_or(3000).to_string();
        let ff = ff_char.unwrap_or("");
        let rew = rew_char.unwrap_or("");
        let pause = pause_char.unwrap_or("");
        let cmd = command::format_command(
            command::CONTROL_STREAM_FILE,
            &[filename, escape_digits, &skip, ff, rew, pause],
        );
        self.send_command(&cmd).await
    }

    /// get a full variable expression, evaluating functions and expressions
    pub async fn get_full_variable(
        &mut self,
        expression: &str,
        channel: Option<&str>,
    ) -> Result<AgiResponse> {
        let cmd = match channel {
            Some(ch) => command::format_command(command::GET_FULL_VARIABLE, &[expression, ch]),
            None => command::format_command(command::GET_FULL_VARIABLE, &[expression]),
        };
        self.send_command(&cmd).await
    }

    /// stream file with playback offset, allowing the caller to interrupt with escape digits
    pub async fn get_option(
        &mut self,
        filename: &str,
        escape_digits: &str,
        timeout_ms: Option<i64>,
    ) -> Result<AgiResponse> {
        let cmd = match timeout_ms {
            Some(t) => {
                let ts = t.to_string();
                command::format_command(command::GET_OPTION, &[filename, escape_digits, &ts])
            }
            None => command::format_command(command::GET_OPTION, &[filename, escape_digits]),
        };
        self.send_command(&cmd).await
    }

    /// execute a dialplan subroutine
    pub async fn gosub(
        &mut self,
        context: &str,
        extension: &str,
        priority: &str,
        args: Option<&str>,
    ) -> Result<AgiResponse> {
        let cmd = match args {
            Some(a) => command::format_command(command::GOSUB, &[context, extension, priority, a]),
            None => command::format_command(command::GOSUB, &[context, extension, priority]),
        };
        self.send_command(&cmd).await
    }

    /// do nothing, used for testing
    pub async fn noop(&mut self) -> Result<AgiResponse> {
        let cmd = command::format_command(command::NOOP, &[]);
        self.send_command(&cmd).await
    }

    /// receive a character from the connected channel
    pub async fn receive_char(&mut self, timeout_ms: i64) -> Result<AgiResponse> {
        let timeout = timeout_ms.to_string();
        let cmd = command::format_command(command::RECEIVE_CHAR, &[&timeout]);
        self.send_command(&cmd).await
    }

    /// receive a text message from the connected channel
    pub async fn receive_text(&mut self, timeout_ms: i64) -> Result<AgiResponse> {
        let timeout = timeout_ms.to_string();
        let cmd = command::format_command(command::RECEIVE_TEXT, &[&timeout]);
        self.send_command(&cmd).await
    }

    /// record audio to a file
    pub async fn record_file(
        &mut self,
        filename: &str,
        format: &str,
        escape_digits: &str,
        timeout_ms: i64,
        beep: bool,
        silence: Option<u32>,
    ) -> Result<AgiResponse> {
        let timeout = timeout_ms.to_string();
        let mut args = vec![filename, format, escape_digits, &timeout];
        let beep_str;
        if beep {
            beep_str = "beep".to_string();
            args.push(&beep_str);
        }
        let silence_str;
        if let Some(s) = silence {
            silence_str = format!("s={s}");
            args.push(&silence_str);
        }
        let cmd = command::format_command(command::RECORD_FILE, &args);
        self.send_command(&cmd).await
    }

    /// say an alphabetic string with escape digits
    pub async fn say_alpha(&mut self, text: &str, escape_digits: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SAY_ALPHA, &[text, escape_digits]);
        self.send_command(&cmd).await
    }

    /// say a date (unix timestamp) with escape digits
    pub async fn say_date(&mut self, date: i64, escape_digits: &str) -> Result<AgiResponse> {
        let d = date.to_string();
        let cmd = command::format_command(command::SAY_DATE, &[&d, escape_digits]);
        self.send_command(&cmd).await
    }

    /// say a date and time (unix timestamp) with escape digits
    pub async fn say_datetime(
        &mut self,
        datetime: i64,
        escape_digits: &str,
        format: Option<&str>,
        timezone: Option<&str>,
    ) -> Result<AgiResponse> {
        let dt = datetime.to_string();
        let mut args = vec![dt.as_str(), escape_digits];
        let fmt;
        if let Some(f) = format {
            fmt = f.to_string();
            args.push(&fmt);
            if let Some(tz) = timezone {
                args.push(tz);
            }
        }
        let cmd = command::format_command(command::SAY_DATETIME, &args);
        self.send_command(&cmd).await
    }

    /// say a string phonetically with escape digits
    pub async fn say_phonetic(&mut self, text: &str, escape_digits: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SAY_PHONETIC, &[text, escape_digits]);
        self.send_command(&cmd).await
    }

    /// say a time (unix timestamp) with escape digits
    pub async fn say_time(&mut self, time: i64, escape_digits: &str) -> Result<AgiResponse> {
        let t = time.to_string();
        let cmd = command::format_command(command::SAY_TIME, &[&t, escape_digits]);
        self.send_command(&cmd).await
    }

    /// send an image to the connected channel
    pub async fn send_image(&mut self, image: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SEND_IMAGE, &[image]);
        self.send_command(&cmd).await
    }

    /// send text to the connected channel
    pub async fn send_text(&mut self, text: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SEND_TEXT, &[text]);
        self.send_command(&cmd).await
    }

    /// set the auto-hangup timer in seconds (0 to disable)
    pub async fn set_autohangup(&mut self, seconds: u32) -> Result<AgiResponse> {
        let s = seconds.to_string();
        let cmd = command::format_command(command::SET_AUTOHANGUP, &[&s]);
        self.send_command(&cmd).await
    }

    /// set the dialplan context for continuation after agi completes
    pub async fn set_context(&mut self, context: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SET_CONTEXT, &[context]);
        self.send_command(&cmd).await
    }

    /// set the dialplan extension for continuation after agi completes
    pub async fn set_extension(&mut self, extension: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SET_EXTENSION, &[extension]);
        self.send_command(&cmd).await
    }

    /// enable or disable music on hold
    pub async fn set_music(&mut self, enabled: bool, class: Option<&str>) -> Result<AgiResponse> {
        let on_off = if enabled { "on" } else { "off" };
        let cmd = match class {
            Some(c) => command::format_command(command::SET_MUSIC, &[on_off, c]),
            None => command::format_command(command::SET_MUSIC, &[on_off]),
        };
        self.send_command(&cmd).await
    }

    /// set the dialplan priority for continuation after agi completes
    pub async fn set_priority(&mut self, priority: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SET_PRIORITY, &[priority]);
        self.send_command(&cmd).await
    }

    /// create a speech recognition object
    pub async fn speech_create(&mut self, engine: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SPEECH_CREATE, &[engine]);
        self.send_command(&cmd).await
    }

    /// destroy the current speech recognition object
    pub async fn speech_destroy(&mut self) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SPEECH_DESTROY, &[]);
        self.send_command(&cmd).await
    }

    /// activate a loaded grammar for recognition
    pub async fn speech_activate_grammar(&mut self, grammar_name: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SPEECH_ACTIVATE_GRAMMAR, &[grammar_name]);
        self.send_command(&cmd).await
    }

    /// deactivate a grammar
    pub async fn speech_deactivate_grammar(&mut self, grammar_name: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SPEECH_DEACTIVATE_GRAMMAR, &[grammar_name]);
        self.send_command(&cmd).await
    }

    /// load a grammar from a file
    pub async fn speech_load_grammar(
        &mut self,
        grammar_name: &str,
        path: &str,
    ) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SPEECH_LOAD_GRAMMAR, &[grammar_name, path]);
        self.send_command(&cmd).await
    }

    /// unload a previously loaded grammar
    pub async fn speech_unload_grammar(&mut self, grammar_name: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SPEECH_UNLOAD_GRAMMAR, &[grammar_name]);
        self.send_command(&cmd).await
    }

    /// play a prompt and perform speech recognition
    pub async fn speech_recognize(
        &mut self,
        prompt: &str,
        timeout_ms: i64,
        offset: Option<i64>,
    ) -> Result<AgiResponse> {
        let t = timeout_ms.to_string();
        let cmd = match offset {
            Some(o) => {
                let os = o.to_string();
                command::format_command(command::SPEECH_RECOGNIZE, &[prompt, &t, &os])
            }
            None => command::format_command(command::SPEECH_RECOGNIZE, &[prompt, &t]),
        };
        self.send_command(&cmd).await
    }

    /// set a speech engine setting
    pub async fn speech_set(&mut self, name: &str, value: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SPEECH_SET, &[name, value]);
        self.send_command(&cmd).await
    }

    /// enable or disable tdd mode on the channel
    pub async fn tdd_mode(&mut self, mode: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::TDD_MODE, &[mode]);
        self.send_command(&cmd).await
    }

    /// break out of async agi
    pub async fn asyncagi_break(&mut self) -> Result<AgiResponse> {
        let cmd = command::format_command(command::ASYNCAGI_BREAK, &[]);
        self.send_command(&cmd).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as TokioBufReader};
    use tokio::net::tcp::OwnedReadHalf;
    use tokio::net::{TcpListener, TcpStream};

    /// create a connected channel pair with server-side reader/writer
    async fn mock_channel() -> (
        AgiChannel,
        TokioBufReader<OwnedReadHalf>,
        OwnedWriteHalf,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let client = TcpStream::connect(addr).await.expect("connect");
        let (server, _) = listener.accept().await.expect("accept");
        let (client_read, client_write) = client.into_split();
        let (server_read, server_write) = server.into_split();
        let channel = AgiChannel::new(BufReader::new(client_read), client_write);
        (channel, TokioBufReader::new(server_read), server_write)
    }

    /// spawn server handler that reads one command and responds with 200 result=0,
    /// returns the command string the client sent
    async fn run_ok(
        mut sr: TokioBufReader<OwnedReadHalf>,
        mut sw: OwnedWriteHalf,
    ) -> String {
        let handle = tokio::spawn(async move {
            let mut cmd = String::new();
            sr.read_line(&mut cmd).await.expect("read command");
            sw.write_all(b"200 result=0\n")
                .await
                .expect("write response");
            sw.flush().await.expect("flush");
            cmd
        });
        handle.await.expect("server task")
    }

    #[tokio::test]
    async fn answer() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        let resp = ch.answer().await.expect("answer");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "ANSWER\n");
        assert_eq!(resp.code, 200);
    }

    #[tokio::test]
    async fn hangup_no_channel() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.hangup(None).await.expect("hangup");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "HANGUP\n");
    }

    #[tokio::test]
    async fn hangup_with_channel() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.hangup(Some("SIP/100")).await.expect("hangup");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "HANGUP SIP/100\n");
    }

    #[tokio::test]
    async fn stream_file_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.stream_file("hello-world", "0123456789")
            .await
            .expect("stream_file");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "STREAM FILE hello-world 0123456789\n");
    }

    #[tokio::test]
    async fn get_data_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.get_data("welcome", 5000, 4).await.expect("get_data");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "GET DATA welcome 5000 4\n");
    }

    #[tokio::test]
    async fn say_digits_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.say_digits("12345", "#").await.expect("say_digits");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SAY DIGITS 12345 #\n");
    }

    #[tokio::test]
    async fn say_number_positive() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.say_number(42, "#").await.expect("say_number");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SAY NUMBER 42 #\n");
    }

    #[tokio::test]
    async fn say_number_negative() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.say_number(-7, "#").await.expect("say_number negative");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SAY NUMBER -7 #\n");
    }

    #[tokio::test]
    async fn set_variable_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.set_variable("MY_VAR", "my_value")
            .await
            .expect("set_variable");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SET VARIABLE MY_VAR my_value\n");
    }

    #[tokio::test]
    async fn get_variable_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.get_variable("CALLERID").await.expect("get_variable");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "GET VARIABLE CALLERID\n");
    }

    #[tokio::test]
    async fn exec_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.exec("Playback", "hello-world").await.expect("exec");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "EXEC Playback hello-world\n");
    }

    #[tokio::test]
    async fn wait_for_digit_with_timeout() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.wait_for_digit(5000).await.expect("wait_for_digit");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "WAIT FOR DIGIT 5000\n");
    }

    #[tokio::test]
    async fn wait_for_digit_infinite() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.wait_for_digit(-1)
            .await
            .expect("wait_for_digit infinite");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "WAIT FOR DIGIT -1\n");
    }

    #[tokio::test]
    async fn channel_status_no_channel() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.channel_status(None).await.expect("channel_status");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "CHANNEL STATUS\n");
    }

    #[tokio::test]
    async fn channel_status_with_channel() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.channel_status(Some("SIP/100"))
            .await
            .expect("channel_status");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "CHANNEL STATUS SIP/100\n");
    }

    #[tokio::test]
    async fn verbose_sends_quoted_message() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.verbose("test message", 3).await.expect("verbose");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "VERBOSE \"test message\" 3\n");
    }

    #[tokio::test]
    async fn set_callerid_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.set_callerid("5551234567")
            .await
            .expect("set_callerid");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SET CALLERID 5551234567\n");
    }

    #[tokio::test]
    async fn database_get_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.database_get("cidname", "5551234567")
            .await
            .expect("database_get");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "DATABASE GET cidname 5551234567\n");
    }

    #[tokio::test]
    async fn database_put_with_quoted_value() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.database_put("cidname", "5551234567", "John Doe")
            .await
            .expect("database_put");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "DATABASE PUT cidname 5551234567 \"John Doe\"\n");
    }

    #[tokio::test]
    async fn database_del_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.database_del("cidname", "5551234567")
            .await
            .expect("database_del");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "DATABASE DEL cidname 5551234567\n");
    }

    #[tokio::test]
    async fn database_deltree_with_key() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.database_deltree("cidname", Some("mykey"))
            .await
            .expect("database_deltree");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "DATABASE DELTREE cidname mykey\n");
    }

    #[tokio::test]
    async fn database_deltree_without_key() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.database_deltree("cidname", None)
            .await
            .expect("database_deltree");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "DATABASE DELTREE cidname\n");
    }

    #[tokio::test]
    async fn control_stream_file_all_params() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.control_stream_file(
            "hello-world",
            "#",
            Some(5000),
            Some("*"),
            Some("0"),
            Some("9"),
        )
        .await
        .expect("control_stream_file");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "CONTROL STREAM FILE hello-world # 5000 * 0 9\n");
    }

    #[tokio::test]
    async fn control_stream_file_defaults() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.control_stream_file("hello-world", "#", None, None, None, None)
            .await
            .expect("control_stream_file defaults");
        let cmd = server.await.expect("server");
        // default skipms 3000, empty strings for optional chars
        assert_eq!(cmd, "CONTROL STREAM FILE hello-world # 3000   \n");
    }

    #[tokio::test]
    async fn get_full_variable_with_channel() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.get_full_variable("${CALLERID(num)}", Some("SIP/100"))
            .await
            .expect("get_full_variable");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "GET FULL VARIABLE ${CALLERID(num)} SIP/100\n");
    }

    #[tokio::test]
    async fn get_full_variable_without_channel() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.get_full_variable("${EXTEN}", None)
            .await
            .expect("get_full_variable");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "GET FULL VARIABLE ${EXTEN}\n");
    }

    #[tokio::test]
    async fn get_option_with_timeout() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.get_option("welcome", "#", Some(5000))
            .await
            .expect("get_option");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "GET OPTION welcome # 5000\n");
    }

    #[tokio::test]
    async fn get_option_without_timeout() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.get_option("welcome", "#", None)
            .await
            .expect("get_option");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "GET OPTION welcome #\n");
    }

    #[tokio::test]
    async fn gosub_with_args() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.gosub("default", "s", "1", Some("arg1,arg2"))
            .await
            .expect("gosub");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "GOSUB default s 1 arg1,arg2\n");
    }

    #[tokio::test]
    async fn gosub_without_args() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.gosub("default", "s", "1", None)
            .await
            .expect("gosub");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "GOSUB default s 1\n");
    }

    #[tokio::test]
    async fn noop_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.noop().await.expect("noop");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "NOOP\n");
    }

    #[tokio::test]
    async fn receive_char_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.receive_char(2000).await.expect("receive_char");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "RECEIVE CHAR 2000\n");
    }

    #[tokio::test]
    async fn receive_text_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.receive_text(3000).await.expect("receive_text");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "RECEIVE TEXT 3000\n");
    }

    #[tokio::test]
    async fn record_file_with_beep_and_silence() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.record_file("recording", "wav", "#", 30000, true, Some(5))
            .await
            .expect("record_file");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "RECORD FILE recording wav # 30000 beep s=5\n");
    }

    #[tokio::test]
    async fn record_file_without_beep_and_silence() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.record_file("recording", "wav", "#", 30000, false, None)
            .await
            .expect("record_file");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "RECORD FILE recording wav # 30000\n");
    }

    #[tokio::test]
    async fn say_alpha_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.say_alpha("hello", "#").await.expect("say_alpha");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SAY ALPHA hello #\n");
    }

    #[tokio::test]
    async fn say_date_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.say_date(1_234_567_890, "#").await.expect("say_date");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SAY DATE 1234567890 #\n");
    }

    #[tokio::test]
    async fn say_datetime_with_format_and_timezone() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.say_datetime(1_234_567_890, "#", Some("IMp"), Some("EST"))
            .await
            .expect("say_datetime");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SAY DATETIME 1234567890 # IMp EST\n");
    }

    #[tokio::test]
    async fn say_datetime_without_format() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.say_datetime(1_234_567_890, "#", None, None)
            .await
            .expect("say_datetime");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SAY DATETIME 1234567890 #\n");
    }

    #[tokio::test]
    async fn say_phonetic_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.say_phonetic("hello", "#")
            .await
            .expect("say_phonetic");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SAY PHONETIC hello #\n");
    }

    #[tokio::test]
    async fn say_time_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.say_time(1_234_567_890, "#").await.expect("say_time");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SAY TIME 1234567890 #\n");
    }

    #[tokio::test]
    async fn send_image_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.send_image("myimage").await.expect("send_image");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SEND IMAGE myimage\n");
    }

    #[tokio::test]
    async fn send_text_quotes_spaces() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.send_text("hello world").await.expect("send_text");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SEND TEXT \"hello world\"\n");
    }

    #[tokio::test]
    async fn set_autohangup_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.set_autohangup(30).await.expect("set_autohangup");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SET AUTOHANGUP 30\n");
    }

    #[tokio::test]
    async fn set_context_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.set_context("default").await.expect("set_context");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SET CONTEXT default\n");
    }

    #[tokio::test]
    async fn set_extension_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.set_extension("100").await.expect("set_extension");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SET EXTENSION 100\n");
    }

    #[tokio::test]
    async fn set_music_enabled_with_class() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.set_music(true, Some("default"))
            .await
            .expect("set_music on");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SET MUSIC on default\n");
    }

    #[tokio::test]
    async fn set_music_disabled_without_class() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.set_music(false, None).await.expect("set_music off");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SET MUSIC off\n");
    }

    #[tokio::test]
    async fn set_priority_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.set_priority("1").await.expect("set_priority");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SET PRIORITY 1\n");
    }

    #[tokio::test]
    async fn speech_create_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.speech_create("flite").await.expect("speech_create");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SPEECH CREATE flite\n");
    }

    #[tokio::test]
    async fn speech_destroy_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.speech_destroy().await.expect("speech_destroy");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SPEECH DESTROY\n");
    }

    #[tokio::test]
    async fn speech_activate_grammar_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.speech_activate_grammar("mygrammar")
            .await
            .expect("speech_activate_grammar");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SPEECH ACTIVATE GRAMMAR mygrammar\n");
    }

    #[tokio::test]
    async fn speech_deactivate_grammar_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.speech_deactivate_grammar("mygrammar")
            .await
            .expect("speech_deactivate_grammar");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SPEECH DEACTIVATE GRAMMAR mygrammar\n");
    }

    #[tokio::test]
    async fn speech_load_grammar_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.speech_load_grammar("mygrammar", "/path/to/grammar")
            .await
            .expect("speech_load_grammar");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SPEECH LOAD GRAMMAR mygrammar /path/to/grammar\n");
    }

    #[tokio::test]
    async fn speech_unload_grammar_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.speech_unload_grammar("mygrammar")
            .await
            .expect("speech_unload_grammar");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SPEECH UNLOAD GRAMMAR mygrammar\n");
    }

    #[tokio::test]
    async fn speech_recognize_with_offset() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.speech_recognize("beep", 5000, Some(1000))
            .await
            .expect("speech_recognize");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SPEECH RECOGNIZE beep 5000 1000\n");
    }

    #[tokio::test]
    async fn speech_recognize_without_offset() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.speech_recognize("beep", 5000, None)
            .await
            .expect("speech_recognize");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SPEECH RECOGNIZE beep 5000\n");
    }

    #[tokio::test]
    async fn speech_set_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.speech_set("name", "value").await.expect("speech_set");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "SPEECH SET name value\n");
    }

    #[tokio::test]
    async fn tdd_mode_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.tdd_mode("on").await.expect("tdd_mode");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "TDD MODE on\n");
    }

    #[tokio::test]
    async fn asyncagi_break_sends_correct_command() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(run_ok(sr, sw));
        ch.asyncagi_break().await.expect("asyncagi_break");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "ASYNCAGI BREAK\n");
    }

    // -- send_command edge cases --

    #[tokio::test]
    async fn send_command_when_hung_up_returns_error() {
        let (mut ch, sr, sw) = mock_channel().await;
        // respond with 511 to set hung_up flag
        let server = tokio::spawn(async move {
            let mut sr = sr;
            let mut sw = sw;
            let mut cmd = String::new();
            sr.read_line(&mut cmd).await.expect("read");
            sw.write_all(b"511 result=-1\n")
                .await
                .expect("write 511");
            sw.flush().await.expect("flush");
        });
        let err = ch.answer().await.expect_err("expected hung up");
        server.await.expect("server");
        assert!(
            matches!(err, AgiError::ChannelHungUp),
            "expected ChannelHungUp from 511, got {err:?}"
        );
        // subsequent call should fail immediately without network io
        let err = ch.answer().await.expect_err("expected hung up on retry");
        assert!(
            matches!(err, AgiError::ChannelHungUp),
            "expected ChannelHungUp on retry, got {err:?}"
        );
    }

    #[tokio::test]
    async fn send_command_on_eof_sets_hung_up() {
        let (mut ch, sr, sw) = mock_channel().await;
        // server reads command then drops connection
        let server = tokio::spawn(async move {
            let mut sr = sr;
            let mut cmd = String::new();
            sr.read_line(&mut cmd).await.expect("read");
            drop(sw);
        });
        let err = ch.answer().await.expect_err("expected hung up on eof");
        server.await.expect("server");
        assert!(
            matches!(err, AgiError::ChannelHungUp),
            "expected ChannelHungUp on eof, got {err:?}"
        );
    }

    #[tokio::test]
    async fn send_command_on_511_response_sets_hung_up() {
        let (mut ch, sr, sw) = mock_channel().await;
        let server = tokio::spawn(async move {
            let mut sr = sr;
            let mut sw = sw;
            let mut cmd = String::new();
            sr.read_line(&mut cmd).await.expect("read");
            sw.write_all(b"511 result=-1\n")
                .await
                .expect("write 511");
            sw.flush().await.expect("flush");
            cmd
        });
        let err = ch.answer().await.expect_err("expected 511 error");
        let cmd = server.await.expect("server");
        assert_eq!(cmd, "ANSWER\n");
        assert!(
            matches!(err, AgiError::ChannelHungUp),
            "expected ChannelHungUp, got {err:?}"
        );
    }
}
