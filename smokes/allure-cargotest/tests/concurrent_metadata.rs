use allure_cargotest::allure_test;
use std::{thread, time::Duration};

#[allure_test]
#[test]
fn metadata_for_first_test_stays_isolated() {
    allure.label("component", "alpha");
    allure.parameter("case", "alpha");
    thread::sleep(Duration::from_millis(100));
    allure.link(
        "https://example.test/alpha",
        Some("alpha-link".to_string()),
        Some("custom".to_string()),
    );
}

#[allure_test]
#[test]
fn metadata_for_second_test_stays_isolated() {
    allure.label("component", "beta");
    allure.parameter("case", "beta");
    thread::sleep(Duration::from_millis(100));
    allure.link(
        "https://example.test/beta",
        Some("beta-link".to_string()),
        Some("custom".to_string()),
    );
}
