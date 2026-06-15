use allure_cargotest::allure_test;
use std::process::{ExitCode, Termination};

struct AsyncTermination {
    succeeds: bool,
}

impl Termination for AsyncTermination {
    fn report(self) -> ExitCode {
        if self.succeeds {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }
    }
}

async fn async_termination(succeeds: bool) -> AsyncTermination {
    tokio::task::yield_now().await;
    AsyncTermination { succeeds }
}

#[allure_test]
#[tokio::test]
async fn returns_async_exit_code_success() -> ExitCode {
    allure.description("Verifies async ExitCode::SUCCESS return values are reported as passed.");
    allure.log_step("return successful async exit code");
    tokio::task::yield_now().await;
    ExitCode::SUCCESS
}

#[allure_test]
#[tokio::test]
async fn returns_async_exit_code_failure() -> ExitCode {
    allure.description("Verifies async ExitCode::FAILURE return values fail Cargo and Allure.");
    allure.log_step("return failing async exit code");
    tokio::task::yield_now().await;
    ExitCode::FAILURE
}

#[allure_test]
#[tokio::test]
async fn returns_async_custom_termination_success() -> AsyncTermination {
    allure.description("Verifies async named custom Termination return values can pass.");
    allure.log_step("return successful async custom termination");
    async_termination(true).await
}

#[allure_test]
#[tokio::test]
async fn returns_async_custom_termination_failure() -> AsyncTermination {
    allure.description("Verifies async named custom Termination return values fail Cargo and Allure.");
    allure.log_step("return failing async custom termination");
    async_termination(false).await
}

#[allure_test]
#[tokio::test]
async fn returns_async_impl_termination_success() -> impl Termination {
    allure.description("Verifies async opaque impl Termination return values can pass.");
    allure.log_step("return successful async opaque termination");
    async_termination(true).await
}

#[allure_test]
#[tokio::test]
async fn returns_async_impl_termination_failure() -> impl Termination {
    allure.description("Verifies async opaque impl Termination return values fail Cargo and Allure.");
    allure.log_step("return failing async opaque termination");
    async_termination(false).await
}
