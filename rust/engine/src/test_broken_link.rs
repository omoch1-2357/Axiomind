//! Test module for CI validation - intentionally broken documentation link
//!
//! This module is used to validate that the validate-docs CI job correctly
//! detects and fails on broken documentation links.
//!
//! **IMPORTANT**: This file should be removed after integration testing is complete.

/// This function contains an intentionally broken documentation link.
///
/// The link references a non-existent type to test CI validation: [`NonExistentType`]
///
/// # Purpose
/// This is a test function to verify that the `validate-docs` CI job fails
/// when broken documentation links are present in the codebase.
pub fn test_function_with_broken_link() {
    // Empty implementation - this is just for documentation testing
}
