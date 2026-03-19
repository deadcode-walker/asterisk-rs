//! Async Rust client for the Asterisk REST Interface (ARI).
//!
//! ARI provides full call control through a REST API combined with
//! a WebSocket event stream for Stasis applications.

pub mod client;
pub mod config;
pub mod error;
pub mod event;
pub mod resources;
pub(crate) mod transport;
pub mod websocket;
pub(crate) mod ws_transport;

pub use client::AriClient;
pub use config::{AriConfig, TransportMode};
pub use error::AriError;
pub use event::{AriEvent, AriMessage};
