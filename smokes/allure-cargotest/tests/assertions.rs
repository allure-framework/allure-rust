use allure_cargotest::{allure_test, log_asserts, step};

#[log_asserts]
fn helper_assertions_are_logged() {
    assert!(true);
}

#[step]
fn step_assertions_are_nested() {
    assert_eq!(1 + 1, 2);
}

#[allure_test]
#[test]
fn logs_passing_assertions() {
    allure.description("Verifies passing standard assertions are emitted as reviewable Allure steps.");
    allure.log_step("run passing assertion matrix");
    assert!(true);
    assert_eq!("actual", "actual");
    assert_ne!("left", "right");

    helper_assertions_are_logged();
    step_assertions_are_nested();
}

#[allure_test]
#[test]
fn logs_failed_assertion_details() {
    allure.description("Verifies a failed standard assertion records actual and expected values in status details.");
    allure.log_step("run assertion that should fail with captured details");
    assert_eq!("actual", "expected");
}
