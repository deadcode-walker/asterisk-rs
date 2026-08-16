// pure unit tests — no network, no servers, no mocks
// tests public API correctness: types, serialization, parsing, error handling
mod unit {
    mod agi;
    mod ami_actions;
    mod ami_codec;
    mod ami_events;
    mod ami_response;
    mod ami_tracker;
    mod ari;
    mod core_tests;
    mod pbx;
}
