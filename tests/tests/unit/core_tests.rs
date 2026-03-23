#![allow(clippy::unwrap_used)]

use std::time::Duration;

use asterisk_rs_core::auth::Credentials;
use asterisk_rs_core::config::{ConnectionState, ReconnectPolicy};
use asterisk_rs_core::error::*;
use asterisk_rs_core::event::{Event, EventBus};
use asterisk_rs_core::types::*;

// =============================================================================
// error tests
// =============================================================================

// compile-time assertion that all error types are send + sync
fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn all_errors_are_send_and_sync() {
    assert_send_sync::<Error>();
    assert_send_sync::<ConnectionError>();
    assert_send_sync::<AuthError>();
    assert_send_sync::<TimeoutError>();
    assert_send_sync::<ProtocolError>();
}

// --- Error (top-level) Display ---

#[test]
fn error_display_connection() {
    let err = Error::Connection(ConnectionError::Closed);
    assert_eq!(
        err.to_string(),
        "connection failed: connection closed unexpectedly"
    );
}

#[test]
fn error_display_auth() {
    let err = Error::Auth(AuthError::InvalidCredentials);
    assert_eq!(
        err.to_string(),
        "authentication failed: invalid credentials"
    );
}

#[test]
fn error_display_timeout() {
    let err = Error::Timeout(TimeoutError::Action {
        elapsed: Duration::from_secs(5),
    });
    assert_eq!(
        err.to_string(),
        "operation timed out: action timed out after 5s"
    );
}

#[test]
fn error_display_protocol() {
    let err = Error::Protocol(ProtocolError::MalformedMessage {
        details: "missing header".into(),
    });
    assert_eq!(
        err.to_string(),
        "protocol error: malformed message: missing header"
    );
}

#[test]
fn error_display_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe broke");
    let err = Error::Io(io_err);
    assert_eq!(err.to_string(), "I/O error: pipe broke");
}

// --- ConnectionError Display ---

#[test]
fn connection_error_connect_failed() {
    let source = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
    let err = ConnectionError::ConnectFailed {
        address: "127.0.0.1:5038".into(),
        source,
    };
    assert_eq!(
        err.to_string(),
        "failed to connect to 127.0.0.1:5038: refused"
    );
}

#[test]
fn connection_error_closed() {
    let err = ConnectionError::Closed;
    assert_eq!(err.to_string(), "connection closed unexpectedly");
}

#[test]
fn connection_error_tls() {
    let err = ConnectionError::Tls("certificate expired".into());
    assert_eq!(err.to_string(), "TLS handshake failed: certificate expired");
}

#[test]
fn connection_error_dns_resolution() {
    let source = std::io::Error::other("NXDOMAIN");
    let err = ConnectionError::DnsResolution {
        host: "pbx.example.com".into(),
        source,
    };
    assert_eq!(
        err.to_string(),
        "DNS resolution failed for pbx.example.com: NXDOMAIN"
    );
}

// --- AuthError Display ---

#[test]
fn auth_error_rejected() {
    let err = AuthError::Rejected {
        reason: "bad password".into(),
    };
    assert_eq!(err.to_string(), "login rejected: bad password");
}

#[test]
fn auth_error_invalid_credentials() {
    let err = AuthError::InvalidCredentials;
    assert_eq!(err.to_string(), "invalid credentials");
}

#[test]
fn auth_error_challenge_failed() {
    let err = AuthError::ChallengeFailed;
    assert_eq!(err.to_string(), "challenge-response failed");
}

// --- TimeoutError Display ---

#[test]
fn timeout_error_action() {
    let err = TimeoutError::Action {
        elapsed: Duration::from_millis(3500),
    };
    assert_eq!(err.to_string(), "action timed out after 3.5s");
}

#[test]
fn timeout_error_connection() {
    let err = TimeoutError::Connection {
        elapsed: Duration::from_secs(30),
    };
    assert_eq!(err.to_string(), "connection timed out after 30s");
}

#[test]
fn timeout_error_sub_millisecond() {
    let err = TimeoutError::Action {
        elapsed: Duration::from_micros(500),
    };
    // Duration debug uses µs for sub-ms values
    assert!(err.to_string().contains("500"));
}

// --- ProtocolError Display ---

#[test]
fn protocol_error_malformed_message() {
    let err = ProtocolError::MalformedMessage {
        details: "unexpected EOF".into(),
    };
    assert_eq!(err.to_string(), "malformed message: unexpected EOF");
}

#[test]
fn protocol_error_unexpected_response() {
    let err = ProtocolError::UnexpectedResponse {
        expected: "Success".into(),
        actual: "Error".into(),
    };
    assert_eq!(
        err.to_string(),
        "unexpected response: expected Success, got Error"
    );
}

#[test]
fn protocol_error_unsupported_version() {
    let err = ProtocolError::UnsupportedVersion {
        version: "99.0".into(),
    };
    assert_eq!(err.to_string(), "unsupported protocol version: 99.0");
}

// --- From impls ---

#[test]
fn from_connection_error() {
    let inner = ConnectionError::Closed;
    let err: Error = inner.into();
    assert!(matches!(err, Error::Connection(ConnectionError::Closed)));
}

#[test]
fn from_auth_error() {
    let inner = AuthError::ChallengeFailed;
    let err: Error = inner.into();
    assert!(matches!(err, Error::Auth(AuthError::ChallengeFailed)));
}

#[test]
fn from_timeout_error() {
    let inner = TimeoutError::Action {
        elapsed: Duration::from_secs(1),
    };
    let err: Error = inner.into();
    assert!(matches!(err, Error::Timeout(TimeoutError::Action { .. })));
}

#[test]
fn from_protocol_error() {
    let inner = ProtocolError::MalformedMessage {
        details: "x".into(),
    };
    let err: Error = inner.into();
    assert!(matches!(
        err,
        Error::Protocol(ProtocolError::MalformedMessage { .. })
    ));
}

#[test]
fn from_io_error() {
    let inner = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
    let err: Error = inner.into();
    assert!(matches!(err, Error::Io(_)));
}

// --- edge cases ---

#[test]
fn connection_error_empty_address() {
    let source = std::io::Error::new(std::io::ErrorKind::InvalidInput, "bad");
    let err = ConnectionError::ConnectFailed {
        address: String::new(),
        source,
    };
    assert_eq!(err.to_string(), "failed to connect to : bad");
}

#[test]
fn auth_error_rejected_empty_reason() {
    let err = AuthError::Rejected {
        reason: String::new(),
    };
    assert_eq!(err.to_string(), "login rejected: ");
}

#[test]
fn protocol_error_empty_details() {
    let err = ProtocolError::MalformedMessage {
        details: String::new(),
    };
    assert_eq!(err.to_string(), "malformed message: ");
}

// =============================================================================
// auth tests
// =============================================================================

#[test]
fn credentials_debug_redacts_secret() {
    let creds = Credentials::new("admin", "s3cret");
    let debug = format!("{creds:?}");
    assert!(debug.contains("admin"));
    assert!(debug.contains("[redacted]"));
    assert!(!debug.contains("s3cret"));
}

#[test]
fn credentials_accessors() {
    let creds = Credentials::new("user", "pass");
    assert_eq!(creds.username(), "user");
    assert_eq!(creds.secret(), "pass");
}

#[test]
fn credentials_clone_preserves_values() {
    let creds = Credentials::new("admin", "secret");
    let cloned = creds.clone();
    assert_eq!(cloned.username(), "admin");
    assert_eq!(cloned.secret(), "secret");
}

#[test]
fn credentials_empty_username_and_secret() {
    let creds = Credentials::new("", "");
    assert_eq!(creds.username(), "");
    assert_eq!(creds.secret(), "");
}

#[test]
fn credentials_unicode_values() {
    let creds = Credentials::new("администратор", "密码🔑");
    assert_eq!(creds.username(), "администратор");
    assert_eq!(creds.secret(), "密码🔑");
}

#[test]
fn credentials_special_characters_in_secret() {
    let secret = "line1\nline2:colon spaces\ttab";
    let creds = Credentials::new("user", secret);
    assert_eq!(creds.secret(), secret);
}

#[test]
fn credentials_very_long_strings() {
    let long_user = "u".repeat(10_000);
    let long_secret = "s".repeat(10_000);
    let creds = Credentials::new(long_user.clone(), long_secret.clone());
    assert_eq!(creds.username(), long_user);
    assert_eq!(creds.secret(), long_secret);
}

#[test]
fn credentials_debug_format_structure() {
    let creds = Credentials::new("testuser", "hunter2");
    let debug = format!("{creds:?}");
    // verify the debug output uses debug_struct format
    assert!(debug.starts_with("Credentials {"));
    assert!(debug.contains("username: \"testuser\""));
    assert!(debug.contains("secret: \"[redacted]\""));
    assert!(debug.ends_with("}"));
    // secret value must never appear
    assert!(!debug.contains("hunter2"));
}

// =============================================================================
// types tests
// =============================================================================

#[test]
fn hangup_cause_round_trip() {
    let cause = HangupCause::NormalClearing;
    assert_eq!(cause.code(), 16);
    assert_eq!(
        HangupCause::from_code(16),
        Some(HangupCause::NormalClearing)
    );
    assert_eq!(cause.to_string(), "normal clearing");
}

#[test]
fn hangup_cause_unknown_code() {
    assert_eq!(HangupCause::from_code(255), None);
}

#[test]
fn channel_state_from_name() {
    assert_eq!(ChannelState::from_str_name("Up"), Some(ChannelState::Up));
    assert_eq!(
        ChannelState::from_str_name("Ring"),
        Some(ChannelState::Ring)
    );
    assert_eq!(ChannelState::from_str_name("bogus"), None);
}

#[test]
fn channel_state_display() {
    assert_eq!(ChannelState::Up.to_string(), "Up");
    assert_eq!(ChannelState::DialingOffhook.to_string(), "Dialing Offhook");
}

#[test]
fn device_state_round_trip() {
    assert_eq!(
        DeviceState::from_str_name("INUSE"),
        Some(DeviceState::InUse)
    );
    assert_eq!(DeviceState::InUse.as_str(), "INUSE");
}

#[test]
fn dial_status_round_trip() {
    assert_eq!(
        DialStatus::from_str_name("NOANSWER"),
        Some(DialStatus::NoAnswer)
    );
    assert_eq!(DialStatus::NoAnswer.as_str(), "NOANSWER");
}

#[test]
fn cdr_disposition_round_trip() {
    assert_eq!(
        CdrDisposition::from_str_name("ANSWERED"),
        Some(CdrDisposition::Answered)
    );
    assert_eq!(CdrDisposition::Answered.as_str(), "ANSWERED");
}

#[test]
fn peer_status_round_trip() {
    assert_eq!(
        PeerStatus::from_str_name("Reachable"),
        Some(PeerStatus::Reachable)
    );
    assert_eq!(PeerStatus::Reachable.as_str(), "Reachable");
}

#[test]
fn queue_strategy_round_trip() {
    assert_eq!(
        QueueStrategy::from_str_name("ringall"),
        Some(QueueStrategy::RingAll)
    );
    assert_eq!(QueueStrategy::RingAll.as_str(), "ringall");
}

#[test]
fn extension_state_round_trip() {
    assert_eq!(ExtensionState::from_code(1), Some(ExtensionState::InUse));
    assert_eq!(ExtensionState::InUse.code(), 1);
}

#[test]
fn agi_status_round_trip() {
    assert_eq!(AgiStatus::from_code(200), Some(AgiStatus::Success));
    assert_eq!(AgiStatus::Success.code(), 200);
}

// compile-time trait assertions
#[allow(dead_code)]
const fn assert_traits<
    T: Copy + Clone + std::cmp::PartialEq + std::cmp::Eq + std::hash::Hash + std::fmt::Debug,
>() {
}

#[test]
fn all_enums_implement_required_traits() {
    assert_traits::<HangupCause>();
    assert_traits::<ChannelState>();
    assert_traits::<DeviceState>();
    assert_traits::<DialStatus>();
    assert_traits::<CdrDisposition>();
    assert_traits::<PeerStatus>();
    assert_traits::<QueueStrategy>();
    assert_traits::<ExtensionState>();
    assert_traits::<AgiStatus>();
}

// --- HangupCause ---

/// (code, variant, description) for every HangupCause variant
const HANGUP_CASES: &[(u32, HangupCause, &str)] = &[
    (0, HangupCause::NotDefined, "not defined"),
    (1, HangupCause::Unallocated, "unallocated number"),
    (
        2,
        HangupCause::NoRouteTransitNet,
        "no route to transit network",
    ),
    (
        3,
        HangupCause::NoRouteDestination,
        "no route to destination",
    ),
    (
        5,
        HangupCause::MisdialledTrunkPrefix,
        "misdialled trunk prefix",
    ),
    (6, HangupCause::ChannelUnacceptable, "channel unacceptable"),
    (
        7,
        HangupCause::CallAwardedDelivered,
        "call awarded and being delivered",
    ),
    (8, HangupCause::PreEmpted, "pre-empted"),
    (
        14,
        HangupCause::NumberPortedNotHere,
        "number ported but not found here",
    ),
    (16, HangupCause::NormalClearing, "normal clearing"),
    (17, HangupCause::UserBusy, "user busy"),
    (18, HangupCause::NoUserResponse, "no user response"),
    (19, HangupCause::NoAnswer, "no answer"),
    (20, HangupCause::SubscriberAbsent, "subscriber absent"),
    (21, HangupCause::CallRejected, "call rejected"),
    (22, HangupCause::NumberChanged, "number changed"),
    (
        23,
        HangupCause::RedirectedToNewDestination,
        "redirected to new destination",
    ),
    (26, HangupCause::AnsweredElsewhere, "answered elsewhere"),
    (
        27,
        HangupCause::DestinationOutOfOrder,
        "destination out of order",
    ),
    (
        28,
        HangupCause::InvalidNumberFormat,
        "invalid number format",
    ),
    (29, HangupCause::FacilityRejected, "facility rejected"),
    (
        30,
        HangupCause::ResponseToStatusEnquiry,
        "response to status enquiry",
    ),
    (31, HangupCause::NormalUnspecified, "normal unspecified"),
    (
        34,
        HangupCause::NormalCircuitCongestion,
        "normal circuit congestion",
    ),
    (38, HangupCause::NetworkOutOfOrder, "network out of order"),
    (
        41,
        HangupCause::NormalTemporaryFailure,
        "normal temporary failure",
    ),
    (42, HangupCause::SwitchCongestion, "switch congestion"),
    (
        43,
        HangupCause::AccessInfoDiscarded,
        "access information discarded",
    ),
    (
        44,
        HangupCause::RequestedChanUnavail,
        "requested channel unavailable",
    ),
    (
        50,
        HangupCause::FacilityNotSubscribed,
        "facility not subscribed",
    ),
    (52, HangupCause::OutgoingCallBarred, "outgoing call barred"),
    (54, HangupCause::IncomingCallBarred, "incoming call barred"),
    (
        57,
        HangupCause::BearerCapabilityNotAuth,
        "bearer capability not authorized",
    ),
    (
        58,
        HangupCause::BearerCapabilityNotAvail,
        "bearer capability not available",
    ),
    (
        65,
        HangupCause::BearerCapabilityNotImpl,
        "bearer capability not implemented",
    ),
    (
        66,
        HangupCause::ChanNotImplemented,
        "channel type not implemented",
    ),
    (
        69,
        HangupCause::FacilityNotImplemented,
        "facility not implemented",
    ),
    (
        81,
        HangupCause::InvalidCallReference,
        "invalid call reference",
    ),
    (
        88,
        HangupCause::IncompatibleDestination,
        "incompatible destination",
    ),
    (
        95,
        HangupCause::InvalidMsgUnspecified,
        "invalid message unspecified",
    ),
    (
        96,
        HangupCause::MandatoryIeMissing,
        "mandatory information element missing",
    ),
    (
        97,
        HangupCause::MessageTypeNonexist,
        "message type nonexistent",
    ),
    (98, HangupCause::WrongMessage, "wrong message"),
    (
        99,
        HangupCause::IeNonexist,
        "information element nonexistent",
    ),
    (
        100,
        HangupCause::InvalidIeContents,
        "invalid information element contents",
    ),
    (101, HangupCause::WrongCallState, "wrong call state"),
    (
        102,
        HangupCause::RecoveryOnTimerExpire,
        "recovery on timer expiry",
    ),
    (
        103,
        HangupCause::MandatoryIeLengthError,
        "mandatory information element length error",
    ),
    (111, HangupCause::ProtocolError, "protocol error"),
    (127, HangupCause::Interworking, "interworking unspecified"),
];

#[test]
fn hangup_cause_from_code_all_variants() {
    for &(code, expected, _) in HANGUP_CASES {
        assert_eq!(
            HangupCause::from_code(code),
            Some(expected),
            "from_code({code}) should return {expected:?}",
        );
    }
}

#[test]
fn hangup_cause_code_round_trip_all_variants() {
    for &(code, variant, _) in HANGUP_CASES {
        assert_eq!(
            variant.code(),
            code,
            "{variant:?}.code() should return {code}",
        );
    }
}

#[test]
fn hangup_cause_description_all_variants() {
    for &(_, variant, desc) in HANGUP_CASES {
        assert_eq!(
            variant.description(),
            desc,
            "{variant:?}.description() mismatch",
        );
        // description must be non-empty
        assert!(!variant.description().is_empty());
    }
}

#[test]
fn hangup_cause_display_matches_description() {
    for &(_, variant, desc) in HANGUP_CASES {
        assert_eq!(
            variant.to_string(),
            desc,
            "Display for {variant:?} should match description()",
        );
    }
}

#[test]
fn hangup_cause_from_code_invalid() {
    for code in [4, 9, 10, 15, 128, 255, u32::MAX] {
        assert_eq!(
            HangupCause::from_code(code),
            None,
            "from_code({code}) should return None",
        );
    }
}

// --- ChannelState ---

const CHANNEL_STATE_CASES: &[(u32, ChannelState, &str)] = &[
    (0, ChannelState::Down, "Down"),
    (1, ChannelState::Reserved, "Rsrvd"),
    (2, ChannelState::OffHook, "OffHook"),
    (3, ChannelState::Dialing, "Dialing"),
    (4, ChannelState::Ring, "Ring"),
    (5, ChannelState::Ringing, "Ringing"),
    (6, ChannelState::Up, "Up"),
    (7, ChannelState::Busy, "Busy"),
    (8, ChannelState::DialingOffhook, "Dialing Offhook"),
    (9, ChannelState::PreRing, "Pre-ring"),
];

#[test]
fn channel_state_from_code_all_variants() {
    for &(code, expected, _) in CHANNEL_STATE_CASES {
        assert_eq!(
            ChannelState::from_code(code),
            Some(expected),
            "from_code({code}) should return {expected:?}",
        );
    }
}

#[test]
fn channel_state_code_round_trip() {
    for &(code, variant, _) in CHANNEL_STATE_CASES {
        assert_eq!(variant.code(), code, "{variant:?}.code() mismatch");
    }
}

#[test]
fn channel_state_from_code_invalid() {
    for code in [10, 100, u32::MAX] {
        assert_eq!(
            ChannelState::from_code(code),
            None,
            "from_code({code}) should return None",
        );
    }
}

#[test]
fn channel_state_from_str_name_all_variants() {
    for &(_, expected, name) in CHANNEL_STATE_CASES {
        assert_eq!(
            ChannelState::from_str_name(name),
            Some(expected),
            "from_str_name({name:?}) should return {expected:?}",
        );
    }
}

#[test]
fn channel_state_from_str_name_invalid() {
    // case sensitive
    assert_eq!(ChannelState::from_str_name("down"), None);
    assert_eq!(ChannelState::from_str_name("UNKNOWN"), None);
    assert_eq!(ChannelState::from_str_name(""), None);
}

#[test]
fn channel_state_display_round_trips_with_from_str_name() {
    for &(_, variant, _) in CHANNEL_STATE_CASES {
        let displayed = variant.to_string();
        assert_eq!(
            ChannelState::from_str_name(&displayed),
            Some(variant),
            "Display -> from_str_name round-trip failed for {variant:?}",
        );
    }
}

// --- DeviceState ---

const DEVICE_STATE_CASES: &[(DeviceState, &str)] = &[
    (DeviceState::Unknown, "UNKNOWN"),
    (DeviceState::NotInUse, "NOT_INUSE"),
    (DeviceState::InUse, "INUSE"),
    (DeviceState::Busy, "BUSY"),
    (DeviceState::Invalid, "INVALID"),
    (DeviceState::Unavailable, "UNAVAILABLE"),
    (DeviceState::Ringing, "RINGING"),
    (DeviceState::RingInUse, "RINGINUSE"),
    (DeviceState::OnHold, "ONHOLD"),
];

#[test]
fn device_state_from_str_name_all_variants() {
    for &(expected, name) in DEVICE_STATE_CASES {
        assert_eq!(
            DeviceState::from_str_name(name),
            Some(expected),
            "from_str_name({name:?}) should return {expected:?}",
        );
    }
}

#[test]
fn device_state_as_str_round_trip() {
    for &(variant, name) in DEVICE_STATE_CASES {
        assert_eq!(variant.as_str(), name, "{variant:?}.as_str() mismatch");
    }
}

#[test]
fn device_state_display_matches_as_str() {
    for &(variant, name) in DEVICE_STATE_CASES {
        assert_eq!(
            variant.to_string(),
            name,
            "Display for {variant:?} should match as_str()",
        );
    }
}

#[test]
fn device_state_from_str_name_invalid() {
    assert_eq!(DeviceState::from_str_name("unknown"), None);
    assert_eq!(DeviceState::from_str_name("InUse"), None);
    assert_eq!(DeviceState::from_str_name(""), None);
}

// --- DialStatus ---

const DIAL_STATUS_CASES: &[(DialStatus, &str)] = &[
    (DialStatus::Answer, "ANSWER"),
    (DialStatus::Busy, "BUSY"),
    (DialStatus::NoAnswer, "NOANSWER"),
    (DialStatus::Cancel, "CANCEL"),
    (DialStatus::Congestion, "CONGESTION"),
    (DialStatus::ChanUnavail, "CHANUNAVAIL"),
    (DialStatus::DontCall, "DONTCALL"),
    (DialStatus::Torture, "TORTURE"),
    (DialStatus::InvalidArgs, "INVALIDARGS"),
    (DialStatus::Unavailable, "UNAVAILABLE"),
];

#[test]
fn dial_status_from_str_name_all_variants() {
    for &(expected, name) in DIAL_STATUS_CASES {
        assert_eq!(
            DialStatus::from_str_name(name),
            Some(expected),
            "from_str_name({name:?}) should return {expected:?}",
        );
    }
}

#[test]
fn dial_status_as_str_round_trip() {
    for &(variant, name) in DIAL_STATUS_CASES {
        assert_eq!(variant.as_str(), name, "{variant:?}.as_str() mismatch");
    }
}

#[test]
fn dial_status_display_matches_as_str() {
    for &(variant, _) in DIAL_STATUS_CASES {
        assert_eq!(
            variant.to_string(),
            variant.as_str(),
            "Display for {variant:?} should match as_str()",
        );
    }
}

#[test]
fn dial_status_from_str_name_invalid() {
    assert_eq!(DialStatus::from_str_name("answer"), None);
    assert_eq!(DialStatus::from_str_name("Busy"), None);
    assert_eq!(DialStatus::from_str_name(""), None);
}

// --- CdrDisposition ---

const CDR_DISPOSITION_CASES: &[(CdrDisposition, &str)] = &[
    (CdrDisposition::NoAnswer, "NO ANSWER"),
    (CdrDisposition::Answered, "ANSWERED"),
    (CdrDisposition::Busy, "BUSY"),
    (CdrDisposition::Failed, "FAILED"),
    (CdrDisposition::Congestion, "CONGESTION"),
];

#[test]
fn cdr_disposition_from_str_name_all_variants() {
    for &(expected, name) in CDR_DISPOSITION_CASES {
        assert_eq!(
            CdrDisposition::from_str_name(name),
            Some(expected),
            "from_str_name({name:?}) should return {expected:?}",
        );
    }
}

#[test]
fn cdr_disposition_as_str_round_trip() {
    for &(variant, name) in CDR_DISPOSITION_CASES {
        assert_eq!(variant.as_str(), name, "{variant:?}.as_str() mismatch");
    }
}

#[test]
fn cdr_disposition_display_matches_as_str() {
    for &(variant, _) in CDR_DISPOSITION_CASES {
        assert_eq!(
            variant.to_string(),
            variant.as_str(),
            "Display for {variant:?} should match as_str()",
        );
    }
}

#[test]
fn cdr_disposition_from_str_name_invalid() {
    assert_eq!(CdrDisposition::from_str_name("no answer"), None);
    assert_eq!(CdrDisposition::from_str_name("Answered"), None);
    assert_eq!(CdrDisposition::from_str_name(""), None);
}

// --- PeerStatus ---

const PEER_STATUS_CASES: &[(PeerStatus, &str)] = &[
    (PeerStatus::Registered, "Registered"),
    (PeerStatus::Unregistered, "Unregistered"),
    (PeerStatus::Reachable, "Reachable"),
    (PeerStatus::Unreachable, "Unreachable"),
    (PeerStatus::Lagged, "Lagged"),
    (PeerStatus::Rejected, "Rejected"),
    (PeerStatus::Unknown, "Unknown"),
];

#[test]
fn peer_status_from_str_name_all_variants() {
    for &(expected, name) in PEER_STATUS_CASES {
        assert_eq!(
            PeerStatus::from_str_name(name),
            Some(expected),
            "from_str_name({name:?}) should return {expected:?}",
        );
    }
}

#[test]
fn peer_status_as_str_round_trip() {
    for &(variant, name) in PEER_STATUS_CASES {
        assert_eq!(variant.as_str(), name, "{variant:?}.as_str() mismatch");
    }
}

#[test]
fn peer_status_display_matches_as_str() {
    for &(variant, _) in PEER_STATUS_CASES {
        assert_eq!(
            variant.to_string(),
            variant.as_str(),
            "Display for {variant:?} should match as_str()",
        );
    }
}

#[test]
fn peer_status_from_str_name_invalid() {
    assert_eq!(PeerStatus::from_str_name("registered"), None);
    assert_eq!(PeerStatus::from_str_name("UNKNOWN"), None);
    assert_eq!(PeerStatus::from_str_name(""), None);
}

// --- QueueStrategy ---

const QUEUE_STRATEGY_CASES: &[(QueueStrategy, &str)] = &[
    (QueueStrategy::RingAll, "ringall"),
    (QueueStrategy::LeastRecent, "leastrecent"),
    (QueueStrategy::FewestCalls, "fewestcalls"),
    (QueueStrategy::Random, "random"),
    (QueueStrategy::RoundRobin, "rrmemory"),
    (QueueStrategy::Linear, "linear"),
    (QueueStrategy::WeightedRandom, "wrandom"),
];

#[test]
fn queue_strategy_from_str_name_all_variants() {
    for &(expected, name) in QUEUE_STRATEGY_CASES {
        assert_eq!(
            QueueStrategy::from_str_name(name),
            Some(expected),
            "from_str_name({name:?}) should return {expected:?}",
        );
    }
}

#[test]
fn queue_strategy_as_str_round_trip() {
    for &(variant, name) in QUEUE_STRATEGY_CASES {
        assert_eq!(variant.as_str(), name, "{variant:?}.as_str() mismatch");
    }
}

#[test]
fn queue_strategy_display_matches_as_str() {
    for &(variant, _) in QUEUE_STRATEGY_CASES {
        assert_eq!(
            variant.to_string(),
            variant.as_str(),
            "Display for {variant:?} should match as_str()",
        );
    }
}

#[test]
fn queue_strategy_from_str_name_invalid() {
    assert_eq!(QueueStrategy::from_str_name("RingAll"), None);
    assert_eq!(QueueStrategy::from_str_name("RANDOM"), None);
    assert_eq!(QueueStrategy::from_str_name(""), None);
}

// --- ExtensionState ---

const EXTENSION_STATE_CASES: &[(i32, ExtensionState, &str)] = &[
    (-2, ExtensionState::Removed, "removed"),
    (-1, ExtensionState::Deactivated, "deactivated"),
    (0, ExtensionState::NotInUse, "not in use"),
    (1, ExtensionState::InUse, "in use"),
    (2, ExtensionState::Busy, "busy"),
    (4, ExtensionState::Unavailable, "unavailable"),
    (8, ExtensionState::Ringing, "ringing"),
    (16, ExtensionState::OnHold, "on hold"),
];

#[test]
fn extension_state_from_code_all_variants() {
    for &(code, expected, _) in EXTENSION_STATE_CASES {
        assert_eq!(
            ExtensionState::from_code(code),
            Some(expected),
            "from_code({code}) should return {expected:?}",
        );
    }
}

#[test]
fn extension_state_code_round_trip() {
    for &(code, variant, _) in EXTENSION_STATE_CASES {
        assert_eq!(variant.code(), code, "{variant:?}.code() mismatch");
    }
}

#[test]
fn extension_state_from_code_other() {
    // bitmask combinations and unrecognized codes map to Other, not None
    for code in [3, 5, 9, 17, 32, -3] {
        assert_eq!(
            ExtensionState::from_code(code),
            Some(ExtensionState::Other(code)),
            "from_code({code}) should return Other({code})",
        );
    }
}

#[test]
fn extension_state_display_all_variants() {
    for &(_, variant, display) in EXTENSION_STATE_CASES {
        assert_eq!(
            variant.to_string(),
            display,
            "Display for {variant:?} mismatch",
        );
    }
}

// --- AgiStatus ---

const AGI_STATUS_CASES: &[(u16, AgiStatus, &str)] = &[
    (200, AgiStatus::Success, "success"),
    (510, AgiStatus::InvalidCommand, "invalid command"),
    (511, AgiStatus::DeadChannel, "dead channel"),
    (520, AgiStatus::EndUsage, "end usage"),
];

#[test]
fn agi_status_from_code_all_variants() {
    for &(code, expected, _) in AGI_STATUS_CASES {
        assert_eq!(
            AgiStatus::from_code(code),
            Some(expected),
            "from_code({code}) should return {expected:?}",
        );
    }
}

#[test]
fn agi_status_code_round_trip() {
    for &(code, variant, _) in AGI_STATUS_CASES {
        assert_eq!(variant.code(), code, "{variant:?}.code() mismatch");
    }
}

#[test]
fn agi_status_from_code_invalid() {
    for code in [0, 100, 201, 512] {
        assert_eq!(
            AgiStatus::from_code(code),
            None,
            "from_code({code}) should return None",
        );
    }
}

#[test]
fn agi_status_display_all_variants() {
    for &(_, variant, display) in AGI_STATUS_CASES {
        assert_eq!(
            variant.to_string(),
            display,
            "Display for {variant:?} mismatch",
        );
    }
}

// =============================================================================
// config tests
// =============================================================================

#[test]
fn exponential_backoff_increases() {
    let mut policy = ReconnectPolicy::exponential(Duration::from_secs(1), Duration::from_secs(60));
    policy.jitter = false;

    assert_eq!(policy.delay_for_attempt(0), Duration::from_secs(1));
    assert_eq!(policy.delay_for_attempt(1), Duration::from_secs(2));
    assert_eq!(policy.delay_for_attempt(2), Duration::from_secs(4));
    assert_eq!(policy.delay_for_attempt(3), Duration::from_secs(8));
}

#[test]
fn exponential_backoff_caps_at_max() {
    let mut policy = ReconnectPolicy::exponential(Duration::from_secs(1), Duration::from_secs(10));
    policy.jitter = false;

    assert_eq!(policy.delay_for_attempt(5), Duration::from_secs(10));
    assert_eq!(policy.delay_for_attempt(100), Duration::from_secs(10));
}

#[test]
fn fixed_policy_constant_delay() {
    let policy = ReconnectPolicy::fixed(Duration::from_secs(5));
    assert_eq!(policy.delay_for_attempt(0), Duration::from_secs(5));
    assert_eq!(policy.delay_for_attempt(10), Duration::from_secs(5));
}

#[test]
fn none_policy_returns_zero() {
    let policy = ReconnectPolicy::none();
    assert_eq!(policy.delay_for_attempt(0), Duration::ZERO);
}

#[test]
fn max_retries_returns_zero_after_exhausted() {
    let mut policy = ReconnectPolicy::exponential(Duration::from_secs(1), Duration::from_secs(60));
    policy.jitter = false;
    policy.max_retries = Some(3);

    assert!(policy.delay_for_attempt(2) > Duration::ZERO);
    assert_eq!(policy.delay_for_attempt(3), Duration::ZERO);
    assert_eq!(policy.delay_for_attempt(100), Duration::ZERO);
}

#[test]
fn jitter_stays_in_range() {
    // jitter_factor is in [0.5, 1.5), so 10s base -> [5s, 15s)
    let policy = ReconnectPolicy::exponential(Duration::from_secs(10), Duration::from_secs(60));
    let delay = policy.delay_for_attempt(0);
    assert!(delay >= Duration::from_secs(5), "delay too low: {delay:?}");
    assert!(delay < Duration::from_secs(15), "delay too high: {delay:?}");
}

#[test]
fn connection_state_display() {
    assert_eq!(ConnectionState::Connected.to_string(), "connected");
    assert_eq!(ConnectionState::Disconnected.to_string(), "disconnected");
    assert_eq!(ConnectionState::Reconnecting.to_string(), "reconnecting");
}

#[test]
fn default_matches_exponential_1s_60s() {
    let default = ReconnectPolicy::default();
    assert_eq!(default.initial_delay, Duration::from_secs(1));
    assert_eq!(default.max_delay, Duration::from_secs(60));
    assert_eq!(default.backoff_factor, 2.0);
    assert!(default.jitter);
    assert!(default.max_retries.is_none());
}

#[test]
fn exponential_custom_backoff_factor() {
    let mut policy = ReconnectPolicy::exponential(Duration::from_secs(1), Duration::from_secs(100));
    policy.jitter = false;
    policy.backoff_factor = 3.0;

    // 1 * 3^0 = 1, 1 * 3^1 = 3, 1 * 3^2 = 9, 1 * 3^3 = 27
    assert_eq!(policy.delay_for_attempt(0), Duration::from_secs(1));
    assert_eq!(policy.delay_for_attempt(1), Duration::from_secs(3));
    assert_eq!(policy.delay_for_attempt(2), Duration::from_secs(9));
    assert_eq!(policy.delay_for_attempt(3), Duration::from_secs(27));
}

#[test]
fn delay_for_attempt_zero_always_returns_initial() {
    let mut policy =
        ReconnectPolicy::exponential(Duration::from_millis(500), Duration::from_secs(30));
    policy.jitter = false;

    // any base^0 = 1, so delay = initial_delay * 1
    assert_eq!(policy.delay_for_attempt(0), Duration::from_millis(500));
}

#[test]
fn delay_for_very_large_attempt_no_panic() {
    let mut policy = ReconnectPolicy::exponential(Duration::from_secs(1), Duration::from_secs(60));
    policy.jitter = false;

    // large attempt wraps as i32, but must not panic
    let delay = policy.delay_for_attempt(u32::MAX - 1);
    assert!(
        delay <= Duration::from_secs(60),
        "delay should not exceed max: {delay:?}"
    );
}

#[test]
fn fixed_policy_constant_for_large_attempts() {
    let policy = ReconnectPolicy::fixed(Duration::from_secs(7));
    assert_eq!(policy.delay_for_attempt(0), Duration::from_secs(7));
    assert_eq!(policy.delay_for_attempt(1000), Duration::from_secs(7));
    assert_eq!(
        policy.delay_for_attempt(u32::MAX - 1),
        Duration::from_secs(7)
    );
}

#[test]
fn with_max_retries_zero_first_attempt_returns_zero() {
    let policy = ReconnectPolicy::exponential(Duration::from_secs(1), Duration::from_secs(60))
        .with_max_retries(0);
    assert_eq!(policy.delay_for_attempt(0), Duration::ZERO);
}

#[test]
fn with_max_retries_one_allows_single_attempt() {
    let mut policy = ReconnectPolicy::exponential(Duration::from_secs(1), Duration::from_secs(60))
        .with_max_retries(1);
    policy.jitter = false;

    assert_eq!(policy.delay_for_attempt(0), Duration::from_secs(1));
    assert_eq!(policy.delay_for_attempt(1), Duration::ZERO);
}

#[test]
fn connection_state_display_connecting() {
    assert_eq!(ConnectionState::Connecting.to_string(), "connecting");
}

#[test]
fn connection_state_partial_eq() {
    // each variant equals itself
    assert_eq!(ConnectionState::Disconnected, ConnectionState::Disconnected);
    assert_eq!(ConnectionState::Connecting, ConnectionState::Connecting);
    assert_eq!(ConnectionState::Connected, ConnectionState::Connected);
    assert_eq!(ConnectionState::Reconnecting, ConnectionState::Reconnecting);

    // variants are not equal to each other
    assert_ne!(ConnectionState::Disconnected, ConnectionState::Connecting);
    assert_ne!(ConnectionState::Connected, ConnectionState::Reconnecting);
    assert_ne!(ConnectionState::Connecting, ConnectionState::Connected);
    assert_ne!(ConnectionState::Disconnected, ConnectionState::Reconnecting);
}

#[test]
fn connection_state_clone_and_debug() {
    let state = ConnectionState::Connected;
    let cloned = state;
    assert_eq!(state, cloned);
    // Debug derive produces variant name
    assert_eq!(format!("{state:?}"), "Connected");
    assert_eq!(
        format!("{:?}", ConnectionState::Disconnected),
        "Disconnected"
    );
}

#[test]
fn zero_duration_policy_no_panic() {
    let mut policy = ReconnectPolicy::exponential(Duration::ZERO, Duration::ZERO);
    policy.jitter = false;

    assert_eq!(policy.delay_for_attempt(0), Duration::ZERO);
    assert_eq!(policy.delay_for_attempt(10), Duration::ZERO);
}

#[test]
fn jitter_produces_values_in_expected_range() {
    // run multiple times to get some statistical coverage
    let policy = ReconnectPolicy::exponential(Duration::from_secs(100), Duration::from_secs(1000));
    for _ in 0..100 {
        let delay = policy.delay_for_attempt(0);
        // base = 100s, jitter range = [0.5 * 100, 1.5 * 100) = [50, 150)
        assert!(
            delay >= Duration::from_secs(50),
            "jitter delay too low: {delay:?}"
        );
        assert!(
            delay < Duration::from_secs(150),
            "jitter delay too high: {delay:?}"
        );
    }
}

// =============================================================================
// event tests
// =============================================================================

#[derive(Debug, Clone)]
struct TestEvent(String);
impl Event for TestEvent {}

#[tokio::test]
async fn publish_and_receive() {
    let bus = EventBus::new(16);
    let mut sub = bus.subscribe();

    bus.publish(TestEvent("hello".into()));

    let event = sub.recv().await.expect("should receive event");
    assert_eq!(event.0, "hello");
}

#[test]
fn publish_to_zero_subscribers_returns_zero() {
    let bus: EventBus<TestEvent> = EventBus::new(16);
    assert_eq!(bus.publish(TestEvent("nobody".into())), 0);
}

#[test]
fn subscriber_count() {
    let bus: EventBus<TestEvent> = EventBus::new(16);
    assert_eq!(bus.subscriber_count(), 0);

    let _sub1 = bus.subscribe();
    assert_eq!(bus.subscriber_count(), 1);

    let _sub2 = bus.subscribe();
    assert_eq!(bus.subscriber_count(), 2);

    drop(_sub1);
    // note: broadcast receiver count doesn't decrease on drop until next send
}

#[test]
fn default_capacity() {
    let bus: EventBus<TestEvent> = EventBus::default();
    // default is 256, just verify it doesn't panic
    bus.publish(TestEvent("test".into()));
}

#[tokio::test]
async fn filtered_subscription_only_matches() {
    let bus = EventBus::new(16);
    let mut filtered = bus.subscribe_filtered(|e: &TestEvent| e.0.starts_with("match"));

    bus.publish(TestEvent("skip-this".into()));
    bus.publish(TestEvent("match-this".into()));
    bus.publish(TestEvent("skip-again".into()));
    bus.publish(TestEvent("match-too".into()));

    let e1 = filtered.recv().await.expect("should get first match");
    assert_eq!(e1.0, "match-this");

    let e2 = filtered.recv().await.expect("should get second match");
    assert_eq!(e2.0, "match-too");
}

#[tokio::test]
async fn subscription_with_filter_conversion() {
    let bus = EventBus::new(16);
    let sub = bus.subscribe();
    let mut filtered = sub.with_filter(|e: &TestEvent| e.0 == "target");

    bus.publish(TestEvent("other".into()));
    bus.publish(TestEvent("target".into()));

    let event = filtered.recv().await.expect("should get target");
    assert_eq!(event.0, "target");
}

#[tokio::test]
async fn capacity_one_bus_works() {
    let bus = EventBus::new(1);
    let mut sub = bus.subscribe();

    bus.publish(TestEvent("single".into()));
    let event = sub
        .recv()
        .await
        .expect("should receive from capacity-1 bus");
    assert_eq!(event.0, "single");
}

#[tokio::test]
async fn multiple_subscribers_receive_same_event() {
    let bus = EventBus::new(16);
    let mut sub1 = bus.subscribe();
    let mut sub2 = bus.subscribe();

    bus.publish(TestEvent("broadcast".into()));

    let e1 = sub1.recv().await.expect("sub1 should receive");
    let e2 = sub2.recv().await.expect("sub2 should receive");
    assert_eq!(e1.0, "broadcast");
    assert_eq!(e2.0, "broadcast");
}

#[tokio::test]
async fn bus_dropped_recv_returns_none() {
    let bus: EventBus<TestEvent> = EventBus::new(16);
    let mut sub = bus.subscribe();

    drop(bus);
    assert!(
        sub.recv().await.is_none(),
        "recv should return None after bus dropped"
    );
}

#[test]
fn publish_returns_receiver_count() {
    let bus = EventBus::new(16);
    let _sub1 = bus.subscribe();
    let _sub2 = bus.subscribe();
    let _sub3 = bus.subscribe();

    let count = bus.publish(TestEvent("counted".into()));
    assert_eq!(count, 3);
}

#[test]
fn event_subscription_debug_format() {
    let bus: EventBus<TestEvent> = EventBus::new(16);
    let sub = bus.subscribe();
    let debug = format!("{sub:?}");
    assert!(
        debug.contains("EventSubscription"),
        "unexpected debug: {debug}"
    );
}

#[test]
fn filtered_subscription_debug_format() {
    let bus: EventBus<TestEvent> = EventBus::new(16);
    let filtered = bus.subscribe_filtered(|_| true);
    let debug = format!("{filtered:?}");
    assert!(
        debug.contains("FilteredSubscription"),
        "unexpected debug: {debug}"
    );
}

#[test]
fn clone_shares_underlying_channel() {
    let bus: EventBus<TestEvent> = EventBus::new(16);
    let bus_clone = bus.clone();

    // subscriber on original receives event published on clone
    let _sub = bus.subscribe();
    let count = bus_clone.publish(TestEvent("from_clone".into()));
    assert_eq!(count, 1, "cloned bus should share the channel");
}

#[tokio::test]
async fn subscribe_filtered_always_false_never_delivers() {
    let bus = EventBus::new(16);
    let mut filtered = bus.subscribe_filtered(|_: &TestEvent| false);

    bus.publish(TestEvent("a".into()));
    bus.publish(TestEvent("b".into()));

    // drop bus so recv will eventually return None instead of hanging
    drop(bus);
    assert!(
        filtered.recv().await.is_none(),
        "always-false filter should never deliver"
    );
}

#[tokio::test]
async fn with_filter_on_existing_subscription() {
    let bus = EventBus::new(16);
    let sub = bus.subscribe();
    let mut filtered = sub.with_filter(|e: &TestEvent| e.0.len() > 3);

    bus.publish(TestEvent("ab".into()));
    bus.publish(TestEvent("abcd".into()));
    bus.publish(TestEvent("xy".into()));
    bus.publish(TestEvent("wxyz".into()));

    let e1 = filtered.recv().await.expect("should get first long event");
    assert_eq!(e1.0, "abcd");

    let e2 = filtered.recv().await.expect("should get second long event");
    assert_eq!(e2.0, "wxyz");
}

// =============================================================================
// event bus lag recovery tests
// =============================================================================

#[tokio::test]
async fn event_bus_subscriber_lag_recovers() {
    let bus = EventBus::<TestEvent>::new(4);
    let mut sub = bus.subscribe();

    // publish 10 events into capacity-4 buffer, causing lag
    for i in 0..10 {
        bus.publish(TestEvent(format!("evt-{i}")));
    }

    // recv should skip lagged events and return one of the later ones
    let event = sub.recv().await;
    assert!(
        event.is_some(),
        "recv should return Some after lag recovery"
    );
}

#[tokio::test]
async fn event_bus_subscriber_lag_count() {
    let bus = EventBus::<TestEvent>::new(4);
    let mut sub = bus.subscribe();

    for i in 0..8 {
        bus.publish(TestEvent(format!("evt-{i}")));
    }

    // exact lag count is broadcast-internal; just verify we get an event back
    let event = sub.recv().await;
    assert!(
        event.is_some(),
        "recv should recover from lag and return Some"
    );
}

#[tokio::test]
async fn event_bus_lag_with_filtered_subscription() {
    let bus = EventBus::<TestEvent>::new(2);
    let mut filtered = bus.subscribe_filtered(|_| true);

    for i in 0..10 {
        bus.publish(TestEvent(format!("evt-{i}")));
    }

    // lag handling lives in EventSubscription::recv, filtered delegates to it
    let event = filtered.recv().await;
    assert!(event.is_some(), "filtered recv should survive lag");
}

#[tokio::test]
async fn event_bus_lag() {
    // capacity-2 bus: at most 2 events survive in the ring buffer
    let bus = EventBus::<TestEvent>::new(2);
    let mut sub = bus.subscribe();

    // publish 5 events without reading — forces lag on the slow subscriber.
    // after all 5 are sent, the buffer retains the 2 most recent: evt-3 and evt-4.
    for i in 0..5u32 {
        bus.publish(TestEvent(format!("evt-{i}")));
    }

    // recv recovers from lag (skips lost events) and returns the oldest surviving event
    let first = sub.recv().await.expect("should recover from lag");
    assert_eq!(
        first.0, "evt-3",
        "first post-lag event should be oldest in buffer"
    );

    // second recv delivers the next buffered event without further lag
    let second = sub
        .recv()
        .await
        .expect("second recv should succeed after lag recovery");
    assert_eq!(
        second.0, "evt-4",
        "second post-lag event should be newest in buffer"
    );
}

// =============================================================================
// reconnect policy adversarial tests
// =============================================================================

#[test]
fn reconnect_policy_large_attempt_no_panic() {
    let mut policy = ReconnectPolicy::exponential(Duration::from_secs(1), Duration::from_secs(60));
    policy.jitter = false;

    // u32::MAX as i32 wraps to -1, powi(-1) = 1/backoff_factor — must not panic
    let _duration = policy.delay_for_attempt(u32::MAX);
}

#[test]
fn reconnect_policy_large_attempt_capped() {
    let mut policy = ReconnectPolicy::exponential(Duration::from_secs(1), Duration::from_secs(60));
    policy.jitter = false;

    let duration = policy.delay_for_attempt(u32::MAX);
    assert!(
        duration <= Duration::from_secs(60),
        "delay must be capped at max_delay, got {duration:?}"
    );
}

#[test]
fn reconnect_policy_zero_initial_delay() {
    let policy = ReconnectPolicy {
        initial_delay: Duration::ZERO,
        max_delay: Duration::from_secs(60),
        backoff_factor: 2.0,
        jitter: false,
        max_retries: None,
    };

    // 0 * anything = 0
    assert_eq!(policy.delay_for_attempt(0), Duration::ZERO);
    assert_eq!(policy.delay_for_attempt(5), Duration::ZERO);
}

#[test]
fn reconnect_policy_attempt_at_max_retries_returns_zero() {
    let mut policy = ReconnectPolicy::exponential(Duration::from_secs(1), Duration::from_secs(60));
    policy.jitter = false;
    policy.max_retries = Some(3);

    // attempt >= max_retries returns ZERO (exhausted)
    assert_eq!(policy.delay_for_attempt(3), Duration::ZERO);

    // attempt below max_retries returns non-zero
    assert_ne!(policy.delay_for_attempt(2), Duration::ZERO);
}

// =============================================================================
// jitter entropy tests
// =============================================================================

#[test]
fn jitter_produces_varying_delays() {
    let policy = ReconnectPolicy::exponential(Duration::from_secs(1), Duration::from_secs(60));
    let delays: Vec<Duration> = (0..20).map(|_| policy.delay_for_attempt(0)).collect();
    // with 20 samples from a range of 1000 discrete values, at least 2 should differ
    let unique = delays
        .iter()
        .collect::<std::collections::HashSet<_>>()
        .len();
    assert!(
        unique > 1,
        "expected varying jitter values, got {unique} unique out of 20"
    );
}
