// integration tests requiring a running Asterisk instance
// every test is ignored so generic workspace/all-features commands cannot contact a PBX
// run with: cargo test-live

mod live_tests {
    mod agi;
    mod ami;
    mod ari;
    mod cross_protocol;
}
