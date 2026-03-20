use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

use crate::command;
use crate::error::{AgiError, Result};
use crate::response::AgiResponse;

/// tracks whether a command round-trip is currently in progress
///
/// used to detect cancellation between write and read: if a caller drops
/// a `send_command` future after the write but before the read completes,
/// the state stays `InFlight` and the next call sees it immediately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChannelState {
    /// ready to accept the next command
    Ready,
    /// write has been sent; waiting for the response line(s)
    InFlight,
    /// a previous I/O error left the stream in an undefined state
    Poisoned,
}

/// high-level interface for sending AGI commands over a connection
pub struct AgiChannel {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
    hung_up: bool,
    state: ChannelState,
}

impl AgiChannel {
    /// create a new AGI channel from split TCP stream halves
    pub fn new(reader: BufReader<OwnedReadHalf>, writer: OwnedWriteHalf) -> Self {
        Self {
            reader,
            writer,
            hung_up: false,
            state: ChannelState::Ready,
        }
    }

    /// send a raw command string and parse the response
    ///
    /// # cancel safety
    ///
    /// this function is **not** cancel-safe. dropping the future after the write
    /// but before the read completes leaves an unread response in the buffer.
    /// subsequent calls will observe `ChannelState::InFlight` and return
    /// `AgiError::CommandInFlight` to prevent reading stale data.
    ///
    /// the command should already be formatted with a trailing newline.
    pub async fn send_command(&mut self, command: &str) -> Result<AgiResponse> {
        match self.state {
            ChannelState::InFlight => return Err(AgiError::CommandInFlight),
            ChannelState::Poisoned => return Err(AgiError::ChannelPoisoned),
            ChannelState::Ready => {}
        }
        if self.hung_up {
            return Err(AgiError::ChannelHungUp);
        }

        // mark in-flight before write so that a cancellation between write and
        // read is visible to the next caller
        self.state = ChannelState::InFlight;

        if let Err(e) = self.writer.write_all(command.as_bytes()).await {
            self.state = ChannelState::Poisoned;
            return Err(AgiError::Io(e));
        }
        if let Err(e) = self.writer.flush().await {
            self.state = ChannelState::Poisoned;
            return Err(AgiError::Io(e));
        }

        let mut line = String::new();
        let bytes_read = match self.reader.read_line(&mut line).await {
            Ok(n) => n,
            Err(e) => {
                self.state = ChannelState::Poisoned;
                return Err(AgiError::Io(e));
            }
        };

        if bytes_read == 0 {
            self.hung_up = true;
            self.state = ChannelState::Ready;
            return Err(AgiError::ChannelHungUp);
        }

        // 520 with a dash is a multiline response — drain all continuation
        // lines until the terminating `520 End of proper usage.` line
        if let Some(stripped) = line.strip_prefix("520-") {
            let first = stripped.trim().to_owned();
            let mut usage = first;
            loop {
                let mut next = String::new();
                let n = match self.reader.read_line(&mut next).await {
                    Ok(n) => n,
                    Err(e) => {
                        self.state = ChannelState::Poisoned;
                        return Err(AgiError::Io(e));
                    }
                };
                if n == 0 {
                    self.hung_up = true;
                    self.state = ChannelState::Ready;
                    return Err(AgiError::ChannelHungUp);
                }
                let trimmed = next.trim();
                if trimmed == "520 End of proper usage." {
                    break;
                }
                if !usage.is_empty() {
                    usage.push('\n');
                }
                usage.push_str(trimmed);
            }
            self.state = ChannelState::Ready;
            return Err(AgiError::CommandFailed {
                code: 520,
                message: usage,
            });
        }

        let response = match AgiResponse::parse(&line) {
            Ok(r) => r,
            Err(e) => {
                self.state = ChannelState::Poisoned;
                return Err(e);
            }
        };

        // 511 means the channel is dead
        if response.code == 511 {
            self.hung_up = true;
            self.state = ChannelState::Ready;
            return Err(AgiError::ChannelHungUp);
        }

        self.state = ChannelState::Ready;
        Ok(response)
    }

    /// answer the channel
    pub async fn answer(&mut self) -> Result<AgiResponse> {
        let cmd = command::format_command(command::ANSWER, &[])?;
        self.send_command(&cmd).await
    }

    /// hang up the channel, optionally specifying which channel to hang up
    pub async fn hangup(&mut self, channel: Option<&str>) -> Result<AgiResponse> {
        let cmd = match channel {
            Some(ch) => command::format_command(command::HANGUP, &[ch])?,
            None => command::format_command(command::HANGUP, &[])?,
        };
        self.send_command(&cmd).await
    }

    /// stream a sound file, allowing the caller to interrupt with escape digits
    pub async fn stream_file(
        &mut self,
        filename: &str,
        escape_digits: &str,
    ) -> Result<AgiResponse> {
        let cmd = command::format_command(command::STREAM_FILE, &[filename, escape_digits])?;
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
        let cmd = command::format_command(command::GET_DATA, &[filename, &timeout, &digits])?;
        self.send_command(&cmd).await
    }

    /// say a digit string with escape digits
    pub async fn say_digits(&mut self, digits: &str, escape_digits: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SAY_DIGITS, &[digits, escape_digits])?;
        self.send_command(&cmd).await
    }

    /// say a number with escape digits
    pub async fn say_number(&mut self, number: i64, escape_digits: &str) -> Result<AgiResponse> {
        let num = number.to_string();
        let cmd = command::format_command(command::SAY_NUMBER, &[&num, escape_digits])?;
        self.send_command(&cmd).await
    }

    /// set a channel variable
    pub async fn set_variable(&mut self, name: &str, value: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SET_VARIABLE, &[name, value])?;
        self.send_command(&cmd).await
    }

    /// get a channel variable
    pub async fn get_variable(&mut self, name: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::GET_VARIABLE, &[name])?;
        self.send_command(&cmd).await
    }

    /// execute an asterisk application
    pub async fn exec(&mut self, application: &str, args: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::EXEC, &[application, args])?;
        self.send_command(&cmd).await
    }

    /// wait for a DTMF digit, -1 for infinite timeout
    pub async fn wait_for_digit(&mut self, timeout_ms: i64) -> Result<AgiResponse> {
        let timeout = timeout_ms.to_string();
        let cmd = command::format_command(command::WAIT_FOR_DIGIT, &[&timeout])?;
        self.send_command(&cmd).await
    }

    /// get the status of a channel
    pub async fn channel_status(&mut self, channel: Option<&str>) -> Result<AgiResponse> {
        let cmd = match channel {
            Some(ch) => command::format_command(command::CHANNEL_STATUS, &[ch])?,
            None => command::format_command(command::CHANNEL_STATUS, &[])?,
        };
        self.send_command(&cmd).await
    }

    /// send a verbose message to the asterisk console
    pub async fn verbose(&mut self, message: &str, level: u8) -> Result<AgiResponse> {
        let lvl = level.to_string();
        let cmd = command::format_command(command::VERBOSE, &[message, &lvl])?;
        self.send_command(&cmd).await
    }

    /// set the caller id for the current channel
    pub async fn set_callerid(&mut self, callerid: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SET_CALLERID, &[callerid])?;
        self.send_command(&cmd).await
    }

    /// get a value from the asterisk database
    pub async fn database_get(&mut self, family: &str, key: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::DATABASE_GET, &[family, key])?;
        self.send_command(&cmd).await
    }

    /// set a value in the asterisk database
    pub async fn database_put(
        &mut self,
        family: &str,
        key: &str,
        value: &str,
    ) -> Result<AgiResponse> {
        let cmd = command::format_command(command::DATABASE_PUT, &[family, key, value])?;
        self.send_command(&cmd).await
    }

    /// delete a key from the asterisk database
    pub async fn database_del(&mut self, family: &str, key: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::DATABASE_DEL, &[family, key])?;
        self.send_command(&cmd).await
    }

    /// delete a family or key tree from the asterisk database
    pub async fn database_deltree(
        &mut self,
        family: &str,
        key: Option<&str>,
    ) -> Result<AgiResponse> {
        let cmd = match key {
            Some(k) => command::format_command(command::DATABASE_DELTREE, &[family, k])?,
            None => command::format_command(command::DATABASE_DELTREE, &[family])?,
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
        )?;
        self.send_command(&cmd).await
    }

    /// get a full variable expression, evaluating functions and expressions
    pub async fn get_full_variable(
        &mut self,
        expression: &str,
        channel: Option<&str>,
    ) -> Result<AgiResponse> {
        let cmd = match channel {
            Some(ch) => command::format_command(command::GET_FULL_VARIABLE, &[expression, ch])?,
            None => command::format_command(command::GET_FULL_VARIABLE, &[expression])?,
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
                command::format_command(command::GET_OPTION, &[filename, escape_digits, &ts])?
            }
            None => command::format_command(command::GET_OPTION, &[filename, escape_digits])?,
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
            Some(a) => command::format_command(command::GOSUB, &[context, extension, priority, a])?,
            None => command::format_command(command::GOSUB, &[context, extension, priority])?,
        };
        self.send_command(&cmd).await
    }

    /// do nothing, used for testing
    pub async fn noop(&mut self) -> Result<AgiResponse> {
        let cmd = command::format_command(command::NOOP, &[])?;
        self.send_command(&cmd).await
    }

    /// receive a character from the connected channel
    pub async fn receive_char(&mut self, timeout_ms: i64) -> Result<AgiResponse> {
        let timeout = timeout_ms.to_string();
        let cmd = command::format_command(command::RECEIVE_CHAR, &[&timeout])?;
        self.send_command(&cmd).await
    }

    /// receive a text message from the connected channel
    pub async fn receive_text(&mut self, timeout_ms: i64) -> Result<AgiResponse> {
        let timeout = timeout_ms.to_string();
        let cmd = command::format_command(command::RECEIVE_TEXT, &[&timeout])?;
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
        let cmd = command::format_command(command::RECORD_FILE, &args)?;
        self.send_command(&cmd).await
    }

    /// say an alphabetic string with escape digits
    pub async fn say_alpha(&mut self, text: &str, escape_digits: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SAY_ALPHA, &[text, escape_digits])?;
        self.send_command(&cmd).await
    }

    /// say a date (unix timestamp) with escape digits
    pub async fn say_date(&mut self, date: i64, escape_digits: &str) -> Result<AgiResponse> {
        let d = date.to_string();
        let cmd = command::format_command(command::SAY_DATE, &[&d, escape_digits])?;
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
        let cmd = command::format_command(command::SAY_DATETIME, &args)?;
        self.send_command(&cmd).await
    }

    /// say a string phonetically with escape digits
    pub async fn say_phonetic(&mut self, text: &str, escape_digits: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SAY_PHONETIC, &[text, escape_digits])?;
        self.send_command(&cmd).await
    }

    /// say a time (unix timestamp) with escape digits
    pub async fn say_time(&mut self, time: i64, escape_digits: &str) -> Result<AgiResponse> {
        let t = time.to_string();
        let cmd = command::format_command(command::SAY_TIME, &[&t, escape_digits])?;
        self.send_command(&cmd).await
    }

    /// send an image to the connected channel
    pub async fn send_image(&mut self, image: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SEND_IMAGE, &[image])?;
        self.send_command(&cmd).await
    }

    /// send text to the connected channel
    pub async fn send_text(&mut self, text: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SEND_TEXT, &[text])?;
        self.send_command(&cmd).await
    }

    /// set the auto-hangup timer in seconds (0 to disable)
    pub async fn set_autohangup(&mut self, seconds: u32) -> Result<AgiResponse> {
        let s = seconds.to_string();
        let cmd = command::format_command(command::SET_AUTOHANGUP, &[&s])?;
        self.send_command(&cmd).await
    }

    /// set the dialplan context for continuation after agi completes
    pub async fn set_context(&mut self, context: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SET_CONTEXT, &[context])?;
        self.send_command(&cmd).await
    }

    /// set the dialplan extension for continuation after agi completes
    pub async fn set_extension(&mut self, extension: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SET_EXTENSION, &[extension])?;
        self.send_command(&cmd).await
    }

    /// enable or disable music on hold
    pub async fn set_music(&mut self, enabled: bool, class: Option<&str>) -> Result<AgiResponse> {
        let on_off = if enabled { "on" } else { "off" };
        let cmd = match class {
            Some(c) => command::format_command(command::SET_MUSIC, &[on_off, c])?,
            None => command::format_command(command::SET_MUSIC, &[on_off])?,
        };
        self.send_command(&cmd).await
    }

    /// set the dialplan priority for continuation after agi completes
    pub async fn set_priority(&mut self, priority: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SET_PRIORITY, &[priority])?;
        self.send_command(&cmd).await
    }

    /// create a speech recognition object
    pub async fn speech_create(&mut self, engine: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SPEECH_CREATE, &[engine])?;
        self.send_command(&cmd).await
    }

    /// destroy the current speech recognition object
    pub async fn speech_destroy(&mut self) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SPEECH_DESTROY, &[])?;
        self.send_command(&cmd).await
    }

    /// activate a loaded grammar for recognition
    pub async fn speech_activate_grammar(&mut self, grammar_name: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SPEECH_ACTIVATE_GRAMMAR, &[grammar_name])?;
        self.send_command(&cmd).await
    }

    /// deactivate a grammar
    pub async fn speech_deactivate_grammar(&mut self, grammar_name: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SPEECH_DEACTIVATE_GRAMMAR, &[grammar_name])?;
        self.send_command(&cmd).await
    }

    /// load a grammar from a file
    pub async fn speech_load_grammar(
        &mut self,
        grammar_name: &str,
        path: &str,
    ) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SPEECH_LOAD_GRAMMAR, &[grammar_name, path])?;
        self.send_command(&cmd).await
    }

    /// unload a previously loaded grammar
    pub async fn speech_unload_grammar(&mut self, grammar_name: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SPEECH_UNLOAD_GRAMMAR, &[grammar_name])?;
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
                command::format_command(command::SPEECH_RECOGNIZE, &[prompt, &t, &os])?
            }
            None => command::format_command(command::SPEECH_RECOGNIZE, &[prompt, &t])?,
        };
        self.send_command(&cmd).await
    }

    /// set a speech engine setting
    pub async fn speech_set(&mut self, name: &str, value: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::SPEECH_SET, &[name, value])?;
        self.send_command(&cmd).await
    }

    /// enable or disable tdd mode on the channel
    pub async fn tdd_mode(&mut self, mode: &str) -> Result<AgiResponse> {
        let cmd = command::format_command(command::TDD_MODE, &[mode])?;
        self.send_command(&cmd).await
    }

    /// break out of async agi
    pub async fn asyncagi_break(&mut self) -> Result<AgiResponse> {
        let cmd = command::format_command(command::ASYNCAGI_BREAK, &[])?;
        self.send_command(&cmd).await
    }
}
