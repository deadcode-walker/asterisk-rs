//! Typed enums for Asterisk domain constants.
//!
//! These types provide compile-time safety for values that travel across AMI,
//! ARI, and AGI wire protocols. Each enum maps one-to-one with the constants
//! defined by Asterisk and, where applicable, the underlying ITU-T Q.931/Q.850
//! specifications.

use std::fmt;

// ---------------------------------------------------------------------------
// HangupCause
// ---------------------------------------------------------------------------

/// Q.931/Q.850 hangup cause codes used across AMI and ARI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(u32)]
pub enum HangupCause {
    /// not defined (code 0)
    NotDefined = 0,
    /// unallocated number (code 1)
    Unallocated = 1,
    /// no route to transit network (code 2)
    NoRouteTransitNet = 2,
    /// no route to destination (code 3)
    NoRouteDestination = 3,
    /// misdialled trunk prefix (code 5)
    MisdialledTrunkPrefix = 5,
    /// channel unacceptable (code 6)
    ChannelUnacceptable = 6,
    /// call awarded and being delivered (code 7)
    CallAwardedDelivered = 7,
    /// pre-empted (code 8)
    PreEmpted = 8,
    /// number ported but not found here (code 14)
    NumberPortedNotHere = 14,
    /// normal call clearing (code 16)
    NormalClearing = 16,
    /// user busy (code 17)
    UserBusy = 17,
    /// no user response (code 18)
    NoUserResponse = 18,
    /// no answer from user (code 19)
    NoAnswer = 19,
    /// subscriber absent (code 20)
    SubscriberAbsent = 20,
    /// call rejected (code 21)
    CallRejected = 21,
    /// number changed (code 22)
    NumberChanged = 22,
    /// redirected to new destination (code 23)
    RedirectedToNewDestination = 23,
    /// answered elsewhere (code 26)
    AnsweredElsewhere = 26,
    /// destination out of order (code 27)
    DestinationOutOfOrder = 27,
    /// invalid number format (code 28)
    InvalidNumberFormat = 28,
    /// facility rejected (code 29)
    FacilityRejected = 29,
    /// response to status enquiry (code 30)
    ResponseToStatusEnquiry = 30,
    /// normal unspecified (code 31)
    NormalUnspecified = 31,
    /// normal circuit congestion (code 34)
    NormalCircuitCongestion = 34,
    /// network out of order (code 38)
    NetworkOutOfOrder = 38,
    /// normal temporary failure (code 41)
    NormalTemporaryFailure = 41,
    /// switch congestion (code 42)
    SwitchCongestion = 42,
    /// access information discarded (code 43)
    AccessInfoDiscarded = 43,
    /// requested channel unavailable (code 44)
    RequestedChanUnavail = 44,
    /// facility not subscribed (code 50)
    FacilityNotSubscribed = 50,
    /// outgoing call barred (code 52)
    OutgoingCallBarred = 52,
    /// incoming call barred (code 54)
    IncomingCallBarred = 54,
    /// bearer capability not authorized (code 57)
    BearerCapabilityNotAuth = 57,
    /// bearer capability not available (code 58)
    BearerCapabilityNotAvail = 58,
    /// bearer capability not implemented (code 65)
    BearerCapabilityNotImpl = 65,
    /// channel type not implemented (code 66)
    ChanNotImplemented = 66,
    /// facility not implemented (code 69)
    FacilityNotImplemented = 69,
    /// invalid call reference (code 81)
    InvalidCallReference = 81,
    /// incompatible destination (code 88)
    IncompatibleDestination = 88,
    /// invalid message unspecified (code 95)
    InvalidMsgUnspecified = 95,
    /// mandatory information element missing (code 96)
    MandatoryIeMissing = 96,
    /// message type nonexistent (code 97)
    MessageTypeNonexist = 97,
    /// wrong message (code 98)
    WrongMessage = 98,
    /// information element nonexistent (code 99)
    IeNonexist = 99,
    /// invalid information element contents (code 100)
    InvalidIeContents = 100,
    /// wrong call state (code 101)
    WrongCallState = 101,
    /// recovery on timer expiry (code 102)
    RecoveryOnTimerExpire = 102,
    /// mandatory information element length error (code 103)
    MandatoryIeLengthError = 103,
    /// protocol error (code 111)
    ProtocolError = 111,
    /// interworking unspecified (code 127)
    Interworking = 127,
}

impl HangupCause {
    /// parse a hangup cause from its numeric code
    pub fn from_code(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::NotDefined),
            1 => Some(Self::Unallocated),
            2 => Some(Self::NoRouteTransitNet),
            3 => Some(Self::NoRouteDestination),
            5 => Some(Self::MisdialledTrunkPrefix),
            6 => Some(Self::ChannelUnacceptable),
            7 => Some(Self::CallAwardedDelivered),
            8 => Some(Self::PreEmpted),
            14 => Some(Self::NumberPortedNotHere),
            16 => Some(Self::NormalClearing),
            17 => Some(Self::UserBusy),
            18 => Some(Self::NoUserResponse),
            19 => Some(Self::NoAnswer),
            20 => Some(Self::SubscriberAbsent),
            21 => Some(Self::CallRejected),
            22 => Some(Self::NumberChanged),
            23 => Some(Self::RedirectedToNewDestination),
            26 => Some(Self::AnsweredElsewhere),
            27 => Some(Self::DestinationOutOfOrder),
            28 => Some(Self::InvalidNumberFormat),
            29 => Some(Self::FacilityRejected),
            30 => Some(Self::ResponseToStatusEnquiry),
            31 => Some(Self::NormalUnspecified),
            34 => Some(Self::NormalCircuitCongestion),
            38 => Some(Self::NetworkOutOfOrder),
            41 => Some(Self::NormalTemporaryFailure),
            42 => Some(Self::SwitchCongestion),
            43 => Some(Self::AccessInfoDiscarded),
            44 => Some(Self::RequestedChanUnavail),
            50 => Some(Self::FacilityNotSubscribed),
            52 => Some(Self::OutgoingCallBarred),
            54 => Some(Self::IncomingCallBarred),
            57 => Some(Self::BearerCapabilityNotAuth),
            58 => Some(Self::BearerCapabilityNotAvail),
            65 => Some(Self::BearerCapabilityNotImpl),
            66 => Some(Self::ChanNotImplemented),
            69 => Some(Self::FacilityNotImplemented),
            81 => Some(Self::InvalidCallReference),
            88 => Some(Self::IncompatibleDestination),
            95 => Some(Self::InvalidMsgUnspecified),
            96 => Some(Self::MandatoryIeMissing),
            97 => Some(Self::MessageTypeNonexist),
            98 => Some(Self::WrongMessage),
            99 => Some(Self::IeNonexist),
            100 => Some(Self::InvalidIeContents),
            101 => Some(Self::WrongCallState),
            102 => Some(Self::RecoveryOnTimerExpire),
            103 => Some(Self::MandatoryIeLengthError),
            111 => Some(Self::ProtocolError),
            127 => Some(Self::Interworking),
            _ => None,
        }
    }

    /// the numeric cause code
    pub fn code(self) -> u32 {
        self as u32
    }

    /// human-readable description
    pub fn description(self) -> &'static str {
        match self {
            Self::NotDefined => "not defined",
            Self::Unallocated => "unallocated number",
            Self::NoRouteTransitNet => "no route to transit network",
            Self::NoRouteDestination => "no route to destination",
            Self::MisdialledTrunkPrefix => "misdialled trunk prefix",
            Self::ChannelUnacceptable => "channel unacceptable",
            Self::CallAwardedDelivered => "call awarded and being delivered",
            Self::PreEmpted => "pre-empted",
            Self::NumberPortedNotHere => "number ported but not found here",
            Self::NormalClearing => "normal clearing",
            Self::UserBusy => "user busy",
            Self::NoUserResponse => "no user response",
            Self::NoAnswer => "no answer",
            Self::SubscriberAbsent => "subscriber absent",
            Self::CallRejected => "call rejected",
            Self::NumberChanged => "number changed",
            Self::RedirectedToNewDestination => "redirected to new destination",
            Self::AnsweredElsewhere => "answered elsewhere",
            Self::DestinationOutOfOrder => "destination out of order",
            Self::InvalidNumberFormat => "invalid number format",
            Self::FacilityRejected => "facility rejected",
            Self::ResponseToStatusEnquiry => "response to status enquiry",
            Self::NormalUnspecified => "normal unspecified",
            Self::NormalCircuitCongestion => "normal circuit congestion",
            Self::NetworkOutOfOrder => "network out of order",
            Self::NormalTemporaryFailure => "normal temporary failure",
            Self::SwitchCongestion => "switch congestion",
            Self::AccessInfoDiscarded => "access information discarded",
            Self::RequestedChanUnavail => "requested channel unavailable",
            Self::FacilityNotSubscribed => "facility not subscribed",
            Self::OutgoingCallBarred => "outgoing call barred",
            Self::IncomingCallBarred => "incoming call barred",
            Self::BearerCapabilityNotAuth => "bearer capability not authorized",
            Self::BearerCapabilityNotAvail => "bearer capability not available",
            Self::BearerCapabilityNotImpl => "bearer capability not implemented",
            Self::ChanNotImplemented => "channel type not implemented",
            Self::FacilityNotImplemented => "facility not implemented",
            Self::InvalidCallReference => "invalid call reference",
            Self::IncompatibleDestination => "incompatible destination",
            Self::InvalidMsgUnspecified => "invalid message unspecified",
            Self::MandatoryIeMissing => "mandatory information element missing",
            Self::MessageTypeNonexist => "message type nonexistent",
            Self::WrongMessage => "wrong message",
            Self::IeNonexist => "information element nonexistent",
            Self::InvalidIeContents => "invalid information element contents",
            Self::WrongCallState => "wrong call state",
            Self::RecoveryOnTimerExpire => "recovery on timer expiry",
            Self::MandatoryIeLengthError => "mandatory information element length error",
            Self::ProtocolError => "protocol error",
            Self::Interworking => "interworking unspecified",
        }
    }
}

impl fmt::Display for HangupCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.description())
    }
}

// ---------------------------------------------------------------------------
// ChannelState
// ---------------------------------------------------------------------------

/// channel state as reported by Asterisk
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(u32)]
pub enum ChannelState {
    /// channel is down and available
    Down = 0,
    /// channel is down, but reserved
    Reserved = 1,
    /// channel is off hook
    OffHook = 2,
    /// digits have been dialed
    Dialing = 3,
    /// remote end is ringing
    Ring = 4,
    /// local end is ringing
    Ringing = 5,
    /// channel is up (answered)
    Up = 6,
    /// line is busy
    Busy = 7,
    /// dialing while offhook
    DialingOffhook = 8,
    /// channel detected incoming call before ring
    PreRing = 9,
}

impl ChannelState {
    /// parse a channel state from its numeric code
    pub fn from_code(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::Down),
            1 => Some(Self::Reserved),
            2 => Some(Self::OffHook),
            3 => Some(Self::Dialing),
            4 => Some(Self::Ring),
            5 => Some(Self::Ringing),
            6 => Some(Self::Up),
            7 => Some(Self::Busy),
            8 => Some(Self::DialingOffhook),
            9 => Some(Self::PreRing),
            _ => None,
        }
    }

    /// the numeric state code
    pub fn code(self) -> u32 {
        self as u32
    }

    /// parse from the string representation used in AMI/ARI
    pub fn from_str_name(s: &str) -> Option<Self> {
        match s {
            "Down" => Some(Self::Down),
            "Rsrvd" => Some(Self::Reserved),
            "OffHook" => Some(Self::OffHook),
            "Dialing" => Some(Self::Dialing),
            "Ring" => Some(Self::Ring),
            "Ringing" => Some(Self::Ringing),
            "Up" => Some(Self::Up),
            "Busy" => Some(Self::Busy),
            "Dialing Offhook" => Some(Self::DialingOffhook),
            "Pre-ring" => Some(Self::PreRing),
            _ => None,
        }
    }
}

impl fmt::Display for ChannelState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Down => "Down",
            Self::Reserved => "Rsrvd",
            Self::OffHook => "OffHook",
            Self::Dialing => "Dialing",
            Self::Ring => "Ring",
            Self::Ringing => "Ringing",
            Self::Up => "Up",
            Self::Busy => "Busy",
            Self::DialingOffhook => "Dialing Offhook",
            Self::PreRing => "Pre-ring",
        };
        f.write_str(s)
    }
}

// ---------------------------------------------------------------------------
// DeviceState
// ---------------------------------------------------------------------------

/// device state values used in device state events and queries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DeviceState {
    /// state is unknown
    Unknown,
    /// device is not in use
    NotInUse,
    /// device is in use
    InUse,
    /// device is busy
    Busy,
    /// device is invalid
    Invalid,
    /// device is unavailable
    Unavailable,
    /// device is ringing
    Ringing,
    /// device is ringing and in use
    RingInUse,
    /// device is on hold
    OnHold,
}

impl DeviceState {
    /// parse from the wire-format string
    pub fn from_str_name(s: &str) -> Option<Self> {
        match s {
            "UNKNOWN" => Some(Self::Unknown),
            "NOT_INUSE" => Some(Self::NotInUse),
            "INUSE" => Some(Self::InUse),
            "BUSY" => Some(Self::Busy),
            "INVALID" => Some(Self::Invalid),
            "UNAVAILABLE" => Some(Self::Unavailable),
            "RINGING" => Some(Self::Ringing),
            "RINGINUSE" => Some(Self::RingInUse),
            "ONHOLD" => Some(Self::OnHold),
            _ => None,
        }
    }

    /// the wire-format string
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "UNKNOWN",
            Self::NotInUse => "NOT_INUSE",
            Self::InUse => "INUSE",
            Self::Busy => "BUSY",
            Self::Invalid => "INVALID",
            Self::Unavailable => "UNAVAILABLE",
            Self::Ringing => "RINGING",
            Self::RingInUse => "RINGINUSE",
            Self::OnHold => "ONHOLD",
        }
    }
}

impl fmt::Display for DeviceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// DialStatus
// ---------------------------------------------------------------------------

/// result of a dial attempt, set in the DIALSTATUS channel variable
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DialStatus {
    /// call was answered
    Answer,
    /// remote end was busy
    Busy,
    /// remote end did not answer
    NoAnswer,
    /// call was cancelled
    Cancel,
    /// congestion encountered
    Congestion,
    /// channel was unavailable
    ChanUnavail,
    /// number on do-not-call list
    DontCall,
    /// number routed to torture IVR
    Torture,
    /// invalid arguments to Dial()
    InvalidArgs,
    /// target was unavailable
    Unavailable,
}

impl DialStatus {
    /// parse from the wire-format string
    pub fn from_str_name(s: &str) -> Option<Self> {
        match s {
            "ANSWER" => Some(Self::Answer),
            "BUSY" => Some(Self::Busy),
            "NOANSWER" => Some(Self::NoAnswer),
            "CANCEL" => Some(Self::Cancel),
            "CONGESTION" => Some(Self::Congestion),
            "CHANUNAVAIL" => Some(Self::ChanUnavail),
            "DONTCALL" => Some(Self::DontCall),
            "TORTURE" => Some(Self::Torture),
            "INVALIDARGS" => Some(Self::InvalidArgs),
            "UNAVAILABLE" => Some(Self::Unavailable),
            _ => None,
        }
    }

    /// the wire-format string
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Answer => "ANSWER",
            Self::Busy => "BUSY",
            Self::NoAnswer => "NOANSWER",
            Self::Cancel => "CANCEL",
            Self::Congestion => "CONGESTION",
            Self::ChanUnavail => "CHANUNAVAIL",
            Self::DontCall => "DONTCALL",
            Self::Torture => "TORTURE",
            Self::InvalidArgs => "INVALIDARGS",
            Self::Unavailable => "UNAVAILABLE",
        }
    }
}

impl fmt::Display for DialStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// CdrDisposition
// ---------------------------------------------------------------------------

/// CDR disposition values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CdrDisposition {
    /// call was not answered
    NoAnswer,
    /// call was answered
    Answered,
    /// remote end was busy
    Busy,
    /// call attempt failed
    Failed,
    /// congestion encountered
    Congestion,
}

impl CdrDisposition {
    /// parse from the wire-format string
    pub fn from_str_name(s: &str) -> Option<Self> {
        match s {
            "NO ANSWER" => Some(Self::NoAnswer),
            "ANSWERED" => Some(Self::Answered),
            "BUSY" => Some(Self::Busy),
            "FAILED" => Some(Self::Failed),
            "CONGESTION" => Some(Self::Congestion),
            _ => None,
        }
    }

    /// the wire-format string
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoAnswer => "NO ANSWER",
            Self::Answered => "ANSWERED",
            Self::Busy => "BUSY",
            Self::Failed => "FAILED",
            Self::Congestion => "CONGESTION",
        }
    }
}

impl fmt::Display for CdrDisposition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// PeerStatus
// ---------------------------------------------------------------------------

/// SIP/PJSIP peer registration status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PeerStatus {
    /// peer is registered
    Registered,
    /// peer is unregistered
    Unregistered,
    /// peer is reachable
    Reachable,
    /// peer is unreachable
    Unreachable,
    /// peer response is lagged
    Lagged,
    /// peer registration was rejected
    Rejected,
    /// peer status is unknown
    Unknown,
}

impl PeerStatus {
    /// parse from the wire-format string
    pub fn from_str_name(s: &str) -> Option<Self> {
        match s {
            "Registered" => Some(Self::Registered),
            "Unregistered" => Some(Self::Unregistered),
            "Reachable" => Some(Self::Reachable),
            "Unreachable" => Some(Self::Unreachable),
            "Lagged" => Some(Self::Lagged),
            "Rejected" => Some(Self::Rejected),
            "Unknown" => Some(Self::Unknown),
            _ => None,
        }
    }

    /// the wire-format string
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Registered => "Registered",
            Self::Unregistered => "Unregistered",
            Self::Reachable => "Reachable",
            Self::Unreachable => "Unreachable",
            Self::Lagged => "Lagged",
            Self::Rejected => "Rejected",
            Self::Unknown => "Unknown",
        }
    }
}

impl fmt::Display for PeerStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// QueueStrategy
// ---------------------------------------------------------------------------

/// queue member selection strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum QueueStrategy {
    /// ring all available members simultaneously
    RingAll,
    /// ring the member least recently called
    LeastRecent,
    /// ring the member with the fewest completed calls
    FewestCalls,
    /// ring a random member
    Random,
    /// round-robin with memory
    RoundRobin,
    /// ring members in the order listed
    Linear,
    /// ring a random member, weighted by penalty
    WeightedRandom,
}

impl QueueStrategy {
    /// parse from the wire-format string
    pub fn from_str_name(s: &str) -> Option<Self> {
        match s {
            "ringall" => Some(Self::RingAll),
            "leastrecent" => Some(Self::LeastRecent),
            "fewestcalls" => Some(Self::FewestCalls),
            "random" => Some(Self::Random),
            "rrmemory" => Some(Self::RoundRobin),
            "linear" => Some(Self::Linear),
            "wrandom" => Some(Self::WeightedRandom),
            _ => None,
        }
    }

    /// the wire-format string
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RingAll => "ringall",
            Self::LeastRecent => "leastrecent",
            Self::FewestCalls => "fewestcalls",
            Self::Random => "random",
            Self::RoundRobin => "rrmemory",
            Self::Linear => "linear",
            Self::WeightedRandom => "wrandom",
        }
    }
}

impl fmt::Display for QueueStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// ExtensionState
// ---------------------------------------------------------------------------

/// extension hint state values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(i32)]
pub enum ExtensionState {
    /// not found or removed
    Removed = -2,
    /// idle, no active calls
    Idle = -1,
    /// in use
    InUse = 1,
    /// busy
    Busy = 2,
    /// unavailable
    Unavailable = 4,
    /// ringing
    Ringing = 8,
    /// on hold
    OnHold = 16,
}

impl ExtensionState {
    /// parse an extension state from its numeric code
    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            -2 => Some(Self::Removed),
            -1 => Some(Self::Idle),
            1 => Some(Self::InUse),
            2 => Some(Self::Busy),
            4 => Some(Self::Unavailable),
            8 => Some(Self::Ringing),
            16 => Some(Self::OnHold),
            _ => None,
        }
    }

    /// the numeric state code
    pub fn code(self) -> i32 {
        self as i32
    }
}

impl fmt::Display for ExtensionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Removed => "removed",
            Self::Idle => "idle",
            Self::InUse => "in use",
            Self::Busy => "busy",
            Self::Unavailable => "unavailable",
            Self::Ringing => "ringing",
            Self::OnHold => "on hold",
        };
        f.write_str(s)
    }
}

// ---------------------------------------------------------------------------
// AgiStatus
// ---------------------------------------------------------------------------

/// AGI response status codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(u16)]
pub enum AgiStatus {
    /// success
    Success = 200,
    /// invalid or unknown command
    InvalidCommand = 510,
    /// channel is dead
    DeadChannel = 511,
    /// end of proper usage for command
    EndUsage = 520,
}

impl AgiStatus {
    /// parse an AGI status from its numeric code
    pub fn from_code(code: u16) -> Option<Self> {
        match code {
            200 => Some(Self::Success),
            510 => Some(Self::InvalidCommand),
            511 => Some(Self::DeadChannel),
            520 => Some(Self::EndUsage),
            _ => None,
        }
    }

    /// the numeric status code
    pub fn code(self) -> u16 {
        self as u16
    }
}

impl fmt::Display for AgiStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Success => "success",
            Self::InvalidCommand => "invalid command",
            Self::DeadChannel => "dead channel",
            Self::EndUsage => "end usage",
        };
        f.write_str(s)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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

    // -----------------------------------------------------------------------
    // compile-time trait assertions
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // HangupCause
    // -----------------------------------------------------------------------

    /// (code, variant, description) for every HangupCause variant
    const HANGUP_CASES: &[(u32, HangupCause, &str)] = &[
        (0, HangupCause::NotDefined, "not defined"),
        (1, HangupCause::Unallocated, "unallocated number"),
        (2, HangupCause::NoRouteTransitNet, "no route to transit network"),
        (3, HangupCause::NoRouteDestination, "no route to destination"),
        (5, HangupCause::MisdialledTrunkPrefix, "misdialled trunk prefix"),
        (6, HangupCause::ChannelUnacceptable, "channel unacceptable"),
        (7, HangupCause::CallAwardedDelivered, "call awarded and being delivered"),
        (8, HangupCause::PreEmpted, "pre-empted"),
        (14, HangupCause::NumberPortedNotHere, "number ported but not found here"),
        (16, HangupCause::NormalClearing, "normal clearing"),
        (17, HangupCause::UserBusy, "user busy"),
        (18, HangupCause::NoUserResponse, "no user response"),
        (19, HangupCause::NoAnswer, "no answer"),
        (20, HangupCause::SubscriberAbsent, "subscriber absent"),
        (21, HangupCause::CallRejected, "call rejected"),
        (22, HangupCause::NumberChanged, "number changed"),
        (23, HangupCause::RedirectedToNewDestination, "redirected to new destination"),
        (26, HangupCause::AnsweredElsewhere, "answered elsewhere"),
        (27, HangupCause::DestinationOutOfOrder, "destination out of order"),
        (28, HangupCause::InvalidNumberFormat, "invalid number format"),
        (29, HangupCause::FacilityRejected, "facility rejected"),
        (30, HangupCause::ResponseToStatusEnquiry, "response to status enquiry"),
        (31, HangupCause::NormalUnspecified, "normal unspecified"),
        (34, HangupCause::NormalCircuitCongestion, "normal circuit congestion"),
        (38, HangupCause::NetworkOutOfOrder, "network out of order"),
        (41, HangupCause::NormalTemporaryFailure, "normal temporary failure"),
        (42, HangupCause::SwitchCongestion, "switch congestion"),
        (43, HangupCause::AccessInfoDiscarded, "access information discarded"),
        (44, HangupCause::RequestedChanUnavail, "requested channel unavailable"),
        (50, HangupCause::FacilityNotSubscribed, "facility not subscribed"),
        (52, HangupCause::OutgoingCallBarred, "outgoing call barred"),
        (54, HangupCause::IncomingCallBarred, "incoming call barred"),
        (57, HangupCause::BearerCapabilityNotAuth, "bearer capability not authorized"),
        (58, HangupCause::BearerCapabilityNotAvail, "bearer capability not available"),
        (65, HangupCause::BearerCapabilityNotImpl, "bearer capability not implemented"),
        (66, HangupCause::ChanNotImplemented, "channel type not implemented"),
        (69, HangupCause::FacilityNotImplemented, "facility not implemented"),
        (81, HangupCause::InvalidCallReference, "invalid call reference"),
        (88, HangupCause::IncompatibleDestination, "incompatible destination"),
        (95, HangupCause::InvalidMsgUnspecified, "invalid message unspecified"),
        (96, HangupCause::MandatoryIeMissing, "mandatory information element missing"),
        (97, HangupCause::MessageTypeNonexist, "message type nonexistent"),
        (98, HangupCause::WrongMessage, "wrong message"),
        (99, HangupCause::IeNonexist, "information element nonexistent"),
        (100, HangupCause::InvalidIeContents, "invalid information element contents"),
        (101, HangupCause::WrongCallState, "wrong call state"),
        (102, HangupCause::RecoveryOnTimerExpire, "recovery on timer expiry"),
        (103, HangupCause::MandatoryIeLengthError, "mandatory information element length error"),
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

    // -----------------------------------------------------------------------
    // ChannelState
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // DeviceState
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // DialStatus
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // CdrDisposition
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // PeerStatus
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // QueueStrategy
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // ExtensionState
    // -----------------------------------------------------------------------

    const EXTENSION_STATE_CASES: &[(i32, ExtensionState, &str)] = &[
        (-2, ExtensionState::Removed, "removed"),
        (-1, ExtensionState::Idle, "idle"),
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
    fn extension_state_from_code_invalid() {
        for code in [0, 3, 5, 32, -3] {
            assert_eq!(
                ExtensionState::from_code(code),
                None,
                "from_code({code}) should return None",
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

    // -----------------------------------------------------------------------
    // AgiStatus
    // -----------------------------------------------------------------------

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
}