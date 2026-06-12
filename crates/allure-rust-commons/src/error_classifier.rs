//! Helpers for mapping Rust panic messages to Allure statuses.

use std::any::Any;

use crate::model::{Status, StatusDetails};

/// Classifies an error message as `failed` for assertion-like failures or `broken` otherwise.
pub fn get_status_from_error(message: &str) -> Status {
    let lowercase = message.to_ascii_lowercase();
    if lowercase.contains("assertion")
        || lowercase.contains("assert")
        || lowercase.contains("comparison failed")
        || lowercase.contains("expected panic")
        || lowercase.contains("panic message mismatch")
    {
        Status::Failed
    } else {
        Status::Broken
    }
}

/// Classifies a message into an Allure status and status details.
pub fn classify_message(message: impl Into<String>) -> (Status, StatusDetails) {
    let message = message.into();
    (
        get_status_from_error(&message),
        StatusDetails {
            message: Some(message),
            trace: None,
            actual: None,
            expected: None,
        },
    )
}

/// Classifies a Rust panic payload into an Allure status and status details.
pub fn classify_panic(payload: &Box<dyn Any + Send>) -> (Status, StatusDetails) {
    let message = if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "panic without string payload".to_string()
    };

    classify_message(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::allure_test;

    #[test]
    fn detects_assertion_messages_as_failed() {
        allure_test(
            module_path!(),
            "detects_assertion_messages_as_failed",
            || {
                assert!(matches!(
                    get_status_from_error("assert_eq! left != right"),
                    Status::Failed
                ));
                assert!(matches!(
                    get_status_from_error("assertion failed: expected true"),
                    Status::Failed
                ));
            },
        );
    }

    #[test]
    fn classifies_non_assertion_messages_as_broken() {
        allure_test(
            module_path!(),
            "classifies_non_assertion_messages_as_broken",
            || {
                assert!(matches!(
                    get_status_from_error("panic without string payload"),
                    Status::Broken
                ));
            },
        );
    }
}
