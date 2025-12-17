//! Domain entities for D-TPRES
//!
//! Pure data structures representing business concepts in the D-TPRES system.
//! All entities follow these principles:
//! - Private fields with constructor validation (DDD)
//! - Getter methods for field access
//! - Zeroize for sensitive cryptographic data
//! - State machine pattern for lifecycle management

pub mod capsule;
pub mod cfrag;
pub mod kfrag;
pub mod secret;
pub mod share;

pub use capsule::Capsule;
pub use cfrag::CFrag;
pub use kfrag::KFrag;
pub use secret::{Secret, SecretState};
pub use share::{EncryptedShareData, ShareCollection};
