//! Test module for CI validation - intentionally failing doctest
//!
//! This module is used to validate that the test CI job correctly
//! detects and fails on failing doctests.
//!
//! **IMPORTANT**: This file should be removed after integration testing is complete.

/// This function contains an intentionally failing doctest.
///
/// # Examples
///
/// ```
/// // This doctest will fail intentionally to test CI validation
/// assert_eq!(1 + 1, 3); // Deliberately wrong assertion
/// ```
///
/// # Purpose
/// This is a test function to verify that the `test` CI job fails
/// when doctests contain errors.
pub fn test_function_with_failing_doctest() {
    // Empty implementation - this is just for doctest testing
}
