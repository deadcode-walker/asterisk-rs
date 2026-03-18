//! Async Rust FastAGI server for the Asterisk Gateway Interface.
//!
//! AGI allows external programs to control Asterisk dialplan execution.
//! This crate provides a FastAGI TCP server that dispatches incoming
//! connections to a user-defined handler.

pub mod channel;
pub mod command;
pub mod error;
pub mod handler;
pub mod request;
pub mod response;
pub mod server;

pub use channel::AgiChannel;
pub use error::AgiError;
pub use handler::AgiHandler;
pub use request::AgiRequest;
pub use server::{AgiServer, ShutdownHandle};
