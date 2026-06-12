use allure_cargotest::allure_test;
use std::{thread, time::Duration};

#[allure_test]
#[test]
fn metadata_for_first_test_stays_isolated() {
    allure.description("Verifies metadata recorded by the alpha test is not mixed with a concurrent test.");
    allure.label("component", "alpha");
    allure.parameter("case", "alpha");
    allure.step("hold alpha test context while the second test can run", || {
        thread::sleep(Duration::from_millis(100));
    });
    allure.log_step("record alpha-specific link metadata");
    allure.link(
        "https://example.test/alpha",
        Some("alpha-link".to_string()),
        Some("custom".to_string()),
    );
}

#[allure_test]
#[test]
fn metadata_for_second_test_stays_isolated() {
    allure.description("Verifies metadata recorded by the beta test is not mixed with a concurrent test.");
    allure.label("component", "beta");
    allure.parameter("case", "beta");
    allure.step("hold beta test context while the first test can run", || {
        thread::sleep(Duration::from_millis(100));
    });
    allure.log_step("record beta-specific link metadata");
    allure.link(
        "https://example.test/beta",
        Some("beta-link".to_string()),
        Some("custom".to_string()),
    );
}
