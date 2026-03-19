#![allow(clippy::unwrap_used)]

use asterisk_rs::pbx::{DialOptions, PbxError};

#[test]
fn test_dial_options_default() {
    let opts = DialOptions::new();
    assert!(opts.caller_id.is_none());
    assert!(opts.timeout_ms.is_none());
    assert!(opts.variables.is_none());
}

#[test]
fn test_dial_options_builder() {
    let opts = DialOptions::new()
        .caller_id("Test <1234>")
        .timeout_ms(30000);

    assert_eq!(opts.caller_id.as_deref(), Some("Test <1234>"));
    assert_eq!(opts.timeout_ms, Some(30000));
}

#[test]
fn test_pbx_error_display() {
    let err = PbxError::Timeout;
    assert_eq!(err.to_string(), "operation timed out");

    let err = PbxError::Disconnected;
    assert_eq!(err.to_string(), "client disconnected");

    let err = PbxError::CallFailed {
        cause: 16,
        cause_txt: "Normal Clearing".to_owned(),
    };
    assert_eq!(err.to_string(), "call failed: 16 (Normal Clearing)");
}
