//! D-TPRES Client Library
//!
//! Provides domain entities, value objects, and repository interfaces for the D-TPRES
//! (Deterministic Threshold Proxy Re-Encryption System).
pub mod adapter;
pub mod domain;
pub mod repositories;
pub mod usecase;

pub use usecase as service;
