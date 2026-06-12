use allure_rust_commons as allure;

#[test]
fn uses_commons_test_runtime() {
    allure::test(|| {
        allure::description("Verifies the macro-free commons test runtime writes metadata, steps, and attachments.");
        allure::feature("Manual runtime");
        allure::parameter("style", "no-macro");
        allure::stage("log from manual runtime");
        allure::log_step("logged from manual runtime");

        allure::stage("attach from manual runtime");
        allure::attachment("runtime.txt", "text/plain", "attached from manual runtime");
    });
}
