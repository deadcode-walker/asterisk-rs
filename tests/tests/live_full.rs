// exhaustive integration tests requiring an owned Asterisk instance
// every test is ignored so generic workspace/all-features commands cannot contact a PBX
// run with: just live-full

mod live_tests {
    mod agi;
    mod ami;
    mod ari;
    mod cross_protocol;
}
