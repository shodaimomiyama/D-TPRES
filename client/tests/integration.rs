//! Integration Tests for FORMIX WorkflowServices
//!
//! This module contains integration tests that verify the complete workflow
//! from PHASE 1 (Secret Sharing) to PHASE 3 (Secret Recovery).

#![allow(
    clippy::uninlined_format_args,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::redundant_clone,
    clippy::unreadable_literal,
    clippy::unnested_or_patterns,
    clippy::unwrap_used,
    clippy::expect_used
)]

mod integration {
    #[allow(deprecated)]
    mod actions_integration_test;
    mod builder_api_test;
    mod secret_recovery_workflow_test;
    mod secret_sharing_workflow_test;
    mod workflow_roundtrip_test;

    #[cfg(feature = "hyperbeam")]
    mod test_hyperbeam_e2e;
}
