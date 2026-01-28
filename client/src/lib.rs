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
