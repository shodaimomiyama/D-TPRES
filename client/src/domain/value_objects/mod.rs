//! Domain Value Objects
//!
//! Value objects are immutable objects that describe characteristics
//! but have no conceptual identity. They are compared by value, not by ID.

mod ids;
mod key_pair;
mod secret_data;
mod symmetric_key;

pub use ids::{CFragId, CapsuleId, KFragId, SecretId, ShareCollectionId};
pub use key_pair::KeyPair;
pub use secret_data::SecretData;
pub use symmetric_key::{SYMMETRIC_KEY_SIZE, SymmetricKey};
