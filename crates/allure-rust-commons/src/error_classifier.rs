use std::any::Any;

use crate::model::{Status, StatusDetails};

pub fn get_status_from_error(message: &str) -> Status {
    let lowercase = message.to_ascii_lowercase();
    if lowercase.contains("assertion")
        || lowercase.contains("assert")
        || lowercase.contains("comparison failed")
    {
        Status::Failed
    } else {
        Status::Broken
    }
}

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

    #[test]
    fn detects_assertion_messages_as_failed() {
        assert!(matches!(
            get_status_from_error("assert_eq! left != right"),
            Status::Failed
        ));
        assert!(matches!(
            get_status_from_error("assertion failed: expected true"),
            Status::Failed
        ));
    }

    #[test]
    fn classifies_non_assertion_messages_as_broken() {
        assert!(matches!(
            get_status_from_error("panic without string payload"),
            Status::Broken
        ));
    }
}
