//! Tael - Terminal-agnostic agent inbox
//!
//! Track AI assistant status across terminal panes.
//! Named after Tael, the purple fairy from Zelda: Majora's Mask.

pub mod config;
pub mod file;
pub mod parse;
pub mod render;
pub mod tui;
pub mod types;

pub use types::{Inbox, InboxItem, Status};

#[cfg(test)]
pub use types::test_utils;
