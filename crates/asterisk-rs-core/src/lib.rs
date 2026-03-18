//! Core types, error framework, and event bus for the asterisk-rs ecosystem.

pub mod auth;
pub mod config;
pub mod error;
pub mod event;
pub mod types;

pub use error::{Error, Result};
