use allure_cargotest::allure_test;

#[allure_test]
#[test]
fn writes_all_labels() {
    allure.label("custom", "v1");
    allure.labels([("team", "qa"), ("component", "billing")]);
    allure.epic("checkout");
    allure.feature("payment");
    allure.story("pay by card");
    allure.suite("api-suite");
    allure.parent_suite("integration");
    allure.sub_suite("card-flows");
    allure.owner("alice");
    allure.severity("critical");
    allure.layer("e2e");
    allure.tags(["smoke", "regression"]);
    allure.links([
        (
            "https://example.test/wiki",
            Some("wiki"),
            Some("custom"),
        ),
        (
            "https://example.test/issue/456",
            Some("issue-456"),
            Some("issue"),
        ),
    ]);
    allure.id("T-42");
}

#[allure_test]
#[test]
fn derives_synthetic_suite_labels_by_default() {
    allure.log_step("synthetic suites are added from module path");
}
