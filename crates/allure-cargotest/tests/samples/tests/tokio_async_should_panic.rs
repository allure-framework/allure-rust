use allure_cargotest::allure_test;

#[allure_test]
#[tokio::test]
#[should_panic]
async fn tokio_should_panic_without_expected_passes() {
    tokio::task::yield_now().await;
    panic!("async boom");
}

#[allure_test]
#[tokio::test]
#[should_panic(expected = "boom")]
async fn tokio_should_panic_with_expected_passes() {
    tokio::task::yield_now().await;
    panic!("async boom goes here");
}

#[allure_test]
#[tokio::test]
#[should_panic(expected = "needle")]
async fn tokio_should_panic_with_expected_mismatch_fails() {
    tokio::task::yield_now().await;
    panic!("different async panic message");
}

#[allure_test]
#[tokio::test]
#[should_panic]
async fn tokio_should_panic_without_panic_fails() {
    tokio::task::yield_now().await;
}
