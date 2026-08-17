// Fast, representative integration tests requiring an owned Asterisk instance.
// Every test stays ignored so generic workspace commands cannot contact a PBX.
// Run with: just live-smoke

#[path = "live_tests/smoke.rs"]
mod smoke;
