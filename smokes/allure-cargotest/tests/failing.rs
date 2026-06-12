use allure_cargotest::allure_test;

#[allure_test]
#[test]
fn still_writes_passed() {
    allure.description("Verifies a passing test in a failing sample still writes its own passed result.");
    allure.label("suite", "failing-sample");
    allure.log_step("passing control test completed before sample failure");
}

#[allure_test]
#[test]
fn fails_with_message() {
    allure.description("Verifies panic failures are recorded with the original panic message.");
    allure.log_step("trigger expected sample panic");
    panic!("expected failure from sample");
}
