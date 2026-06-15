use allure_cargotest::allure_test;

#[allure_test]
#[test]
fn selected_by_name() {
    allure.description("Verifies selector-based test plans can include a test by fullName.");
    allure.log_step("selected by fullName when the test plan matches this test");
}

#[allure_test(id = "A-2")]
#[test]
fn selected_by_id() {
    allure.description("Verifies selector-based test plans can target the test's explicit Allure ID.");
    allure.log_step("explicit Allure ID metadata is available for selection");
}

#[allure_test]
#[test]
fn selected_extra() {
    allure.description("Verifies unmatched tests still run when no valid test plan is active.");
    allure.log_step("extra test is available for no-plan and malformed-plan runs");
}
