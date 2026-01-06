//! D-TPRES Client Library
//!
//! Provides domain entities, value objects, and repository interfaces for the D-TPRES
//! (Deterministic Threshold Proxy Re-Encryption System).

// Allow non-critical lints at crate level during development
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::disallowed_names)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::significant_drop_tightening)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::implicit_clone)]
#![allow(clippy::unused_self)]
#![allow(clippy::let_and_return)]
#![allow(clippy::single_char_pattern)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::enum_variant_names)]
#![allow(clippy::range_plus_one)]
#![allow(clippy::use_self)]

pub mod domain;
pub mod repositories;
pub mod usecase;

pub use usecase as service;
