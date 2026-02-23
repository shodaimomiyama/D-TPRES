//! SecretRepository trait definition
//!
//! Repository interface for Secret entity (aggregate root) persistence operations.

use async_trait::async_trait;

use crate::domain::entities::Secret;
use crate::domain::value_objects::SecretId;

use super::Repository;

/// Repository interface for Secret entity
///
/// Secret is the aggregate root in FORMIX, managing references to related entities
/// (ShareCollection, Capsule, KFrag). This repository handles Secret persistence
/// with state transition support (Initialized → Split → Distributed → Recovered).
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait SecretRepository: Repository<Secret, SecretId> {}

/// Repository interface for Secret entity (WASM version)
#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait SecretRepository: Repository<Secret, SecretId> {}
