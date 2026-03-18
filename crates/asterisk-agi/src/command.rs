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
}
