use allure_cargotest::allure_test;

#[allure_test]
#[test]
fn writes_parameters() {
    allure.description("Verifies simple runtime parameters are serialized in insertion order.");
    allure.parameter("browser", "firefox");
    allure.parameter("retries", "2");
    allure.log_step("browser and retry parameters were recorded");
}
