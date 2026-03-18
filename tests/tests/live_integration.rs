// integration tests requiring a running Asterisk instance
// run with: cargo test -p asterisk-rs-tests --test live_integration --features integration
#![cfg(feature = "integration")]

mod live_tests {
    pub mod agi;
    pub mod ami;
    pub mod ari;
}
