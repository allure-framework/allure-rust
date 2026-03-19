use allure_cargotest::allure_test;

#[allure_test]
#[test]
fn computes_test_case_id_from_full_name() {
    allure.log_step("uses generated testCaseId");
}

#[allure_test]
#[test]
fn allows_runtime_override_for_test_case_id() {
    allure.test_case_id("runtime-overridden-test-case-id");
}
