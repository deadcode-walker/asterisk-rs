// standard AGI command names
pub const ANSWER: &str = "ANSWER";
pub const HANGUP: &str = "HANGUP";
pub const STREAM_FILE: &str = "STREAM FILE";
pub const GET_DATA: &str = "GET DATA";
pub const SAY_DIGITS: &str = "SAY DIGITS";
pub const SAY_NUMBER: &str = "SAY NUMBER";
pub const SET_VARIABLE: &str = "SET VARIABLE";
pub const GET_VARIABLE: &str = "GET VARIABLE";
pub const EXEC: &str = "EXEC";
pub const WAIT_FOR_DIGIT: &str = "WAIT FOR DIGIT";
pub const CHANNEL_STATUS: &str = "CHANNEL STATUS";
pub const VERBOSE: &str = "VERBOSE";
pub const SET_CALLERID: &str = "SET CALLERID";
pub const DATABASE_GET: &str = "DATABASE GET";
pub const DATABASE_PUT: &str = "DATABASE PUT";
pub const DATABASE_DEL: &str = "DATABASE DEL";
pub const CONTROL_STREAM_FILE: &str = "CONTROL STREAM FILE";
pub const DATABASE_DELTREE: &str = "DATABASE DELTREE";
pub const GET_FULL_VARIABLE: &str = "GET FULL VARIABLE";
pub const GET_OPTION: &str = "GET OPTION";
pub const GOSUB: &str = "GOSUB";
pub const NOOP: &str = "NOOP";
pub const RECEIVE_CHAR: &str = "RECEIVE CHAR";
pub const RECEIVE_TEXT: &str = "RECEIVE TEXT";
pub const RECORD_FILE: &str = "RECORD FILE";
pub const SAY_ALPHA: &str = "SAY ALPHA";
pub const SAY_DATE: &str = "SAY DATE";
pub const SAY_DATETIME: &str = "SAY DATETIME";
pub const SAY_PHONETIC: &str = "SAY PHONETIC";
pub const SAY_TIME: &str = "SAY TIME";
pub const SEND_IMAGE: &str = "SEND IMAGE";
pub const SEND_TEXT: &str = "SEND TEXT";
pub const SET_AUTOHANGUP: &str = "SET AUTOHANGUP";
pub const SET_CONTEXT: &str = "SET CONTEXT";
pub const SET_EXTENSION: &str = "SET EXTENSION";
pub const SET_MUSIC: &str = "SET MUSIC";
pub const SET_PRIORITY: &str = "SET PRIORITY";
pub const SPEECH_ACTIVATE_GRAMMAR: &str = "SPEECH ACTIVATE GRAMMAR";
pub const SPEECH_CREATE: &str = "SPEECH CREATE";
pub const SPEECH_DEACTIVATE_GRAMMAR: &str = "SPEECH DEACTIVATE GRAMMAR";
pub const SPEECH_DESTROY: &str = "SPEECH DESTROY";
pub const SPEECH_LOAD_GRAMMAR: &str = "SPEECH LOAD GRAMMAR";
pub const SPEECH_RECOGNIZE: &str = "SPEECH RECOGNIZE";
pub const SPEECH_SET: &str = "SPEECH SET";
pub const SPEECH_UNLOAD_GRAMMAR: &str = "SPEECH UNLOAD GRAMMAR";
pub const TDD_MODE: &str = "TDD MODE";
pub const ASYNCAGI_BREAK: &str = "ASYNCAGI BREAK";

/// format an AGI command string with proper quoting
///
/// arguments containing spaces or double quotes are wrapped in double quotes,
/// with embedded double quotes escaped as `\"`
pub fn format_command(name: &str, args: &[&str]) -> String {
    let mut cmd = String::from(name);

    for arg in args {
        cmd.push(' ');
        if arg.contains(' ') || arg.contains('"') {
            cmd.push('"');
            for ch in arg.chars() {
                if ch == '"' {
                    cmd.push('\\');
                }
                cmd.push(ch);
            }
            cmd.push('"');
        } else {
            cmd.push_str(arg);
        }
    }

    cmd.push('\n');
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_simple_command() {
        assert_eq!(format_command(ANSWER, &[]), "ANSWER\n");
    }

    #[test]
    fn format_command_with_args() {
        assert_eq!(
            format_command(STREAM_FILE, &["hello-world", "#"]),
            "STREAM FILE hello-world #\n"
        );
    }

    #[test]
    fn format_command_with_spaces_in_arg() {
        assert_eq!(
            format_command(VERBOSE, &["hello world", "1"]),
            "VERBOSE \"hello world\" 1\n"
        );
    }

    #[test]
    fn format_command_with_embedded_quotes() {
        assert_eq!(
            format_command(VERBOSE, &["say \"hi\"", "1"]),
            "VERBOSE \"say \\\"hi\\\"\" 1\n"
        );
    }

    #[test]
    fn format_hangup_with_optional_channel() {
        assert_eq!(
            format_command(HANGUP, &["SIP/100-00000001"]),
            "HANGUP SIP/100-00000001\n"
        );
    }

    #[test]
    fn format_record_file_command() {
        let cmd = format_command(RECORD_FILE, &["myfile", "wav", "#", "5000"]);
        assert_eq!(cmd, "RECORD FILE myfile wav # 5000\n");
    }

    #[test]
    fn format_database_get_command() {
        let cmd = format_command(DATABASE_GET, &["cidname", "12125551234"]);
        assert_eq!(cmd, "DATABASE GET cidname 12125551234\n");
    }

    #[test]
    fn format_gosub_command() {
        let cmd = format_command(GOSUB, &["default", "s", "1"]);
        assert_eq!(cmd, "GOSUB default s 1\n");
    }

    #[test]
    fn format_say_alpha_command() {
        let cmd = format_command(SAY_ALPHA, &["hello", "#"]);
        assert_eq!(cmd, "SAY ALPHA hello #\n");
    }

    #[test]
    fn format_speech_create_command() {
        let cmd = format_command(SPEECH_CREATE, &["lumenvox"]);
        assert_eq!(cmd, "SPEECH CREATE lumenvox\n");
    }

    #[test]
    fn format_set_callerid_command() {
        let cmd = format_command(SET_CALLERID, &["\"John\" <1234>"]);
        assert_eq!(cmd, "SET CALLERID \"\\\"John\\\" <1234>\"\n");
    }

    #[test]
    fn format_set_music_command() {
        let cmd = format_command(SET_MUSIC, &["on", "default"]);
        assert_eq!(cmd, "SET MUSIC on default\n");
    }
}
