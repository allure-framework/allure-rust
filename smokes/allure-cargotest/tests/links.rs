use allure_cargotest::allure_test;

#[allure_test]
#[test]
fn writes_links() {
    allure.description("Verifies generic, issue, and TMS link helpers are serialized with the expected types.");
    allure.log_step("record documentation link");
    allure.link(
        "https://example.test/docs",
        Some("docs".to_string()),
        Some("custom".to_string()),
    );
    allure.log_step("record issue and TMS links");
    allure.issue("ISSUE-123", "https://example.test/issue/123");
    allure.tms("TMS-456", "https://example.test/tms/456");
}
