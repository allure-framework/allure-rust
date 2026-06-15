use allure_cargotest::allure_test;
use std::{error::Error, fmt};

struct AsyncSampleError(&'static str);

impl fmt::Debug for AsyncSampleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AsyncSampleError: {}", self.0)
    }
}

impl fmt::Display for AsyncSampleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for AsyncSampleError {}

async fn write_async_sample_row(succeeds: bool) -> Result<(), AsyncSampleError> {
    tokio::task::yield_now().await;
    if succeeds {
        Ok(())
    } else {
        Err(AsyncSampleError("simulated async database write failed"))
    }
}

#[allure_test]
#[tokio::test]
async fn returns_ok_async_result() -> Result<(), AsyncSampleError> {
    allure.description("Verifies async Result-returning tests can use question-mark flow and pass.");
    allure.log_step("write async sample row successfully");
    write_async_sample_row(true).await?;
    Ok(())
}

#[allure_test]
#[tokio::test]
async fn returns_err_async_result() -> Result<(), AsyncSampleError> {
    allure.description("Verifies async Result::Err test outcomes are reported before Cargo fails the test.");
    allure.log_step("write async sample row with a simulated database error");
    write_async_sample_row(false).await?;
    Ok(())
}
