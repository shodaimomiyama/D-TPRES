//! Integration Tests for D-TPRES WorkflowServices
//!
//! This module contains integration tests that verify the complete workflow
//! from PHASE 1 (Secret Sharing) to PHASE 3 (Secret Recovery).

mod integration {
    mod secret_recovery_workflow_test;
    mod secret_sharing_workflow_test;
    mod workflow_roundtrip_test;
}
