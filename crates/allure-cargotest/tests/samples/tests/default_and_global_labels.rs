use allure_cargotest::allure_test;

#[allure_test]
#[test]
fn writes_default_and_global_labels() {
    allure.log_step("default labels are automatically added");
}
