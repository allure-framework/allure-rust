use allure_cargotest::allure_test;
use std::{error::Error, fmt};

type QueryResult<T> = Result<T, SampleDbError>;

struct SampleDbError(&'static str);

impl fmt::Debug for SampleDbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SampleDbError: {}", self.0)
    }
}

impl fmt::Display for SampleDbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for SampleDbError {}

fn write_sample_row(succeeds: bool) -> QueryResult<()> {
    if succeeds {
        Ok(())
    } else {
        Err(SampleDbError("simulated database write failed"))
    }
}

#[allure_test]
#[test]
fn returns_ok_query_result() -> QueryResult<()> {
    allure.description("Verifies Result-returning tests can use question-mark flow and pass.");
    allure.log_step("write sample row successfully");
    write_sample_row(true)?;
    Ok(())
}

#[allure_test]
#[test]
fn returns_err_query_result() -> QueryResult<()> {
    allure.description("Verifies Result::Err test outcomes are reported before Cargo fails the test.");
    allure.log_step("write sample row with a simulated database error");
    write_sample_row(false)?;
    Ok(())
}
