use allure_cargotest::allure_test;

#[allure_test]
#[tokio::test]
#[should_panic]
async fn tokio_should_panic_without_expected_passes() {
    allure.description("Verifies async should_panic passes when any panic occurs after an await.");
    tokio::task::yield_now().await;
    allure.log_step("trigger async panic for unqualified should_panic");
    panic!("async boom");
}

#[allure_test]
#[tokio::test]
#[should_panic(expected = "boom")]
async fn tokio_should_panic_with_expected_passes() {
    allure.description("Verifies async should_panic(expected) passes when the panic message contains the expected substring.");
    tokio::task::yield_now().await;
    allure.log_step("trigger async panic containing expected substring");
    panic!("async boom goes here");
}

#[allure_test]
#[tokio::test]
#[should_panic(expected = "needle")]
async fn tokio_should_panic_with_expected_mismatch_fails() {
    allure.description("Verifies async should_panic(expected) fails when the panic message misses the expected substring.");
    tokio::task::yield_now().await;
    allure.log_step("trigger async panic with mismatched message");
    panic!("different async panic message");
}

#[allure_test]
#[tokio::test]
#[should_panic]
async fn tokio_should_panic_without_panic_fails() {
    allure.description("Verifies async should_panic fails when the async body completes without panicking.");
    tokio::task::yield_now().await;
    allure.log_step("complete async body without panic to exercise failure reporting");
}
