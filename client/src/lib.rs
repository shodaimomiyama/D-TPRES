#![allow(
    clippy::missing_const_for_fn,
    clippy::unused_self,
    clippy::significant_drop_tightening,
    clippy::implicit_clone,
    clippy::similar_names,
    clippy::doc_markdown,
    clippy::unnecessary_wraps,
    clippy::use_self,
    clippy::uninlined_format_args,
    clippy::trivially_copy_pass_by_ref,
    clippy::redundant_clone,
    clippy::default_constructed_unit_structs,
    clippy::single_char_pattern,
    clippy::if_not_else,
    clippy::items_after_statements,
    clippy::option_if_let_else,
    clippy::redundant_else,
    clippy::manual_string_new,
    clippy::single_match_else,
    clippy::default_trait_access,
    clippy::redundant_closure_for_method_calls,
    clippy::range_plus_one,
    clippy::collection_is_never_read,
    clippy::manual_let_else,
    clippy::exhaustive_structs,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation
)]

//! D-TPRES Client Library
//!
//! Provides domain entities, value objects, and repository interfaces for the D-TPRES
//! (Deterministic Threshold Proxy Re-Encryption System).
//!
//! # Architecture
//! The library follows a 6-layer Clean Architecture:
//! ```text
//! Actions → Controller → UseCase → Domain
//!                ↓
//!           Repository ← Adapter (implements)
//! ```
//!
//! # Quick Start
//! ```rust,ignore
//! use dtpres_client::actions::{DefaultActionsContainer, ShareOptions};
//!
//! // Create container
//! let container = DefaultActionsContainer::new();
//!
//! // Generate keys
//! let (owner_sk, owner_pk) = container.generate_keypair()?;
//! let (requester_sk, requester_pk) = container.generate_keypair()?;
//!
//! // Share a secret
//! let result = container.share(
//!     b"my secret".to_vec(),
//!     3, 5,  // 3-of-5 threshold
//!     owner_sk,
//!     owner_pk,
//!     requester_pk,
//!     "owner_process".to_string(),
//!     None,
//! )?;
//!
//! // Recover the secret (requires storage implementation)
//! let recovered = container.recover(
//!     result.secret_id.as_str(),
//!     requester_sk,
//!     "requester_process".to_string(),
//!     None,
//! )?;
//! ```

pub mod actions;
pub mod adapter;
pub mod controller;
pub mod domain;
pub mod repositories;
pub mod usecase;

pub use usecase as service;
