//! Async Rust client for the Asterisk REST Interface (ARI).
//!
//! ARI provides full call control through a REST API combined with
//! a WebSocket event stream for Stasis applications.

pub mod client;
pub mod config;
pub mod error;
pub mod event;
pub mod resources;
pub mod websocket;

pub use client::AriClient;
pub use config::AriConfig;
pub use error::AriError;
pub use event::{AriEvent, AriMessage};
