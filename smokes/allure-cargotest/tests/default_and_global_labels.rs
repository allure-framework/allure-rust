use allure_cargotest::allure_test;

#[allure_test]
#[test]
fn writes_default_and_global_labels() {
    allure.description("Verifies runtime labels, package labels, module labels, and env labels are merged into the result.");
    allure.log_step("default labels are automatically added");
}
