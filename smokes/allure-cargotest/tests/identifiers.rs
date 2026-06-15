use allure_cargotest::allure_test;

#[allure_test]
#[test]
fn computes_test_case_id_from_full_name() {
    allure.description("Verifies the adapter derives a stable testCaseId from the generated fullName.");
    allure.log_step("uses generated testCaseId");
}

#[allure_test]
#[test]
fn allows_runtime_override_for_test_case_id() {
    allure.description("Verifies runtime code can override the generated testCaseId when a stable external identity is needed.");
    allure.test_case_id("runtime-overridden-test-case-id");
    allure.log_step("runtime testCaseId override was recorded");
}
