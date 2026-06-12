use allure_cargotest::allure_test;

#[allure_test]
#[test]
#[should_panic]
fn should_panic_without_expected_passes() {
    allure.description("Verifies an unqualified should_panic test is reported as passed when any panic occurs.");
    allure.log_step("trigger panic for unqualified should_panic");
    panic!("boom");
}

#[allure_test]
#[test]
#[should_panic(expected = "boom")]
fn should_panic_with_expected_passes() {
    allure.description("Verifies should_panic(expected) is reported as passed when the panic message contains the expected substring.");
    allure.log_step("trigger panic containing expected substring");
    panic!("boom goes here");
}

#[allure_test]
#[test]
#[should_panic(expected = "needle")]
fn should_panic_with_expected_mismatch_fails() {
    allure.description("Verifies should_panic(expected) is reported as failed when the panic message misses the expected substring.");
    allure.log_step("trigger panic with mismatched message");
    panic!("different panic message");
}

#[allure_test]
#[test]
#[should_panic]
fn should_panic_without_panic_fails() {
    allure.description("Verifies should_panic is reported as failed when the test body does not panic.");
    allure.log_step("complete without panic to exercise failure reporting");
}
