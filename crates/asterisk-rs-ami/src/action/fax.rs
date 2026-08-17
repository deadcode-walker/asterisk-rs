//! Fax-related AMI actions.

use super::AmiAction;

/// Get information about a fax session.
pub struct FaxSessionAction {
    pub session_number: String,
}

impl AmiAction for FaxSessionAction {
    fn action_name(&self) -> &str {
        "FAXSession"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("SessionNumber".into(), self.session_number.clone())]
    }
}

/// List active fax sessions.
pub struct FaxSessionsAction;

impl AmiAction for FaxSessionsAction {
    fn action_name(&self) -> &str {
        "FAXSessions"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// Get fax statistics.
pub struct FaxStatsAction;

impl AmiAction for FaxStatsAction {
    fn action_name(&self) -> &str {
        "FAXStats"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

#[deprecated(since = "0.8.0", note = "use FaxSessionAction")]
pub type FAXSessionAction = FaxSessionAction;
#[deprecated(since = "0.8.0", note = "use FaxSessionsAction")]
pub type FAXSessionsAction = FaxSessionsAction;
#[allow(non_upper_case_globals)]
#[deprecated(since = "0.8.0", note = "use FaxSessionsAction")]
pub const FAXSessionsAction: FaxSessionsAction = FaxSessionsAction;
#[deprecated(since = "0.8.0", note = "use FaxStatsAction")]
pub type FAXStatsAction = FaxStatsAction;
#[allow(non_upper_case_globals)]
#[deprecated(since = "0.8.0", note = "use FaxStatsAction")]
pub const FAXStatsAction: FaxStatsAction = FaxStatsAction;
