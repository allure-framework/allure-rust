use allure_cargotest::allure_test;

#[allure_test]
#[test]
fn writes_parameters() {
    allure.parameter("browser", "firefox");
    allure.parameter("retries", "2");
}
