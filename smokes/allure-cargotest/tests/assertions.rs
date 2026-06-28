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
    let user_id = 42;
    let input = [7, 13];
    let is_valid = true;
    assert!(true);
    assert_eq!("actual", "actual");
    assert_eq!(
        user_id,
        42,
        "expected feature flag to be enabled for user_id={user_id}"
    );
    assert_eq!(
        is_valid,
        true,
        "validation result was wrong for input {:?}",
        input
    );
    assert_ne!("left", "right");

    helper_assertions_are_logged();
    step_assertions_are_nested();
}

#[allure_test]
#[test]
fn logs_failed_assertion_details() {
    allure.description("Verifies a failed standard assertion records actual and expected values in status details.");
    allure.log_step("run assertion that should fail with captured details");
    let input = [404, 500];
    assert_eq!(
        "actual",
        "expected",
        "validation result was wrong for input {:?}",
        input
    );
}
