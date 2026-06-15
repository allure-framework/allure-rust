use allure_cargotest::{allure_test, step};
use std::time::Duration;

#[step(name = "async helper step")]
async fn async_helper_step() {
    tokio::time::sleep(Duration::from_millis(1)).await;
}

#[allure_test(name = "Async custom name", id = "ASYNC-1")]
#[tokio::test]
async fn writes_tokio_async_metadata() {
    allure.description("Verifies Allure metadata remains available across awaits on the current-thread Tokio runtime.");
    allure.label("component", "tokio-current-thread");
    tokio::time::sleep(Duration::from_millis(1)).await;
    allure.parameter("phase", "after-await");
    async_helper_step().await;
    allure.log_step("current-thread async context preserved after helper step");
    allure.attachment("async.txt", "text/plain", "hello from async test");
}

#[allure_test]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn writes_tokio_multi_thread_metadata_after_await() {
    allure.description("Verifies Allure metadata and steps remain available after awaits on a multi-thread Tokio runtime.");
    tokio::task::yield_now().await;
    allure.label("component", "tokio-multi-thread");
    allure.stage("direct async stage");
    tokio::task::yield_now().await;
    allure.log_step("multi-thread async context preserved after yield");
    async_helper_step().await;
}
