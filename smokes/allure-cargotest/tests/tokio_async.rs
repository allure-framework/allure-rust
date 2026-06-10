use allure_cargotest::{allure_test, step};
use std::time::Duration;

#[step(name = "async helper step")]
async fn async_helper_step() {
    tokio::time::sleep(Duration::from_millis(1)).await;
}

#[allure_test(name = "Async custom name", id = "ASYNC-1")]
#[tokio::test]
async fn writes_tokio_async_metadata() {
    allure.label("component", "tokio-current-thread");
    tokio::time::sleep(Duration::from_millis(1)).await;
    allure.parameter("phase", "after-await");
    async_helper_step().await;
    allure.attachment("async.txt", "text/plain", "hello from async test");
}

#[allure_test]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn writes_tokio_multi_thread_metadata_after_await() {
    tokio::task::yield_now().await;
    allure.label("component", "tokio-multi-thread");
    {
        let _direct = allure.step("direct async step");
        tokio::task::yield_now().await;
    }
    async_helper_step().await;
}
