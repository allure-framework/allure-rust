use allure_cargotest::allure_test;

#[allure_test]
#[test]
fn writes_links() {
    allure.link(
        "https://example.test/docs",
        Some("docs".to_string()),
        Some("custom".to_string()),
    );
    allure.issue("ISSUE-123", "https://example.test/issue/123");
    allure.tms("TMS-456", "https://example.test/tms/456");
}
