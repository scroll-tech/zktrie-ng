//! Traits, helpers, and type definitions for key-value databases.
//!
//! This module provides a trait for key-value databases, as well as some
//! helper types and functions for working with key-value databases.
mod kv;
pub use kv::*;

mod shared;
pub use shared::*;

mod update;
pub use update::*;
