//! Async Rust client for the Asterisk Manager Interface (AMI).
//!
//! AMI is a TCP-based protocol for monitoring and controlling Asterisk PBX.
//! This crate provides a fully async client with typed actions, events,
//! automatic reconnection, and MD5 challenge-response authentication.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use asterisk_rs_ami::AmiClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = AmiClient::builder()
//!         .host("127.0.0.1")
//!         .port(5038)
//!         .credentials("admin", "secret")
//!         .build()
//!         .await?;
//!
//!     let response = client.ping().await?;
//!     println!("pong: {:?}", response);
//!     Ok(())
//! }
//! ```

pub mod action;
pub mod client;
pub mod codec;
pub mod connection;
pub mod error;
pub mod event;
pub mod response;

pub use client::{AmiClient, AmiClientBuilder};
pub use error::AmiError;
pub use event::AmiEvent;
