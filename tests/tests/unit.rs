// pure unit tests — no network, no servers, no mocks
// tests public API correctness: types, serialization, parsing, error handling
mod unit {
    pub mod agi;
    pub mod ami_actions;
    pub mod ami_codec;
    pub mod ami_events;
    pub mod ami_response;
    pub mod ari;
    pub mod core_tests;
    pub mod ami_tracker;
    pub mod pbx;
}
