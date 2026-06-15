use allure_cargotest::allure_test;
use std::process::{ExitCode, Termination};

struct SampleTermination {
    succeeds: bool,
}

impl Termination for SampleTermination {
    fn report(self) -> ExitCode {
        if self.succeeds {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }
    }
}

#[allure_test]
#[test]
fn returns_exit_code_success() -> ExitCode {
    allure.description("Verifies ExitCode::SUCCESS return values are reported as passed.");
    allure.log_step("return successful exit code");
    ExitCode::SUCCESS
}

#[allure_test]
#[test]
fn returns_exit_code_failure() -> ExitCode {
    allure.description("Verifies ExitCode::FAILURE return values are reported before Cargo fails the test.");
    allure.log_step("return failing exit code");
    ExitCode::FAILURE
}

#[allure_test]
#[test]
fn returns_custom_termination_success() -> SampleTermination {
    allure.description("Verifies named custom Termination return values can pass.");
    allure.log_step("return successful custom termination");
    SampleTermination { succeeds: true }
}

#[allure_test]
#[test]
fn returns_custom_termination_failure() -> SampleTermination {
    allure.description("Verifies named custom Termination return values can fail Cargo and Allure.");
    allure.log_step("return failing custom termination");
    SampleTermination { succeeds: false }
}

#[allure_test]
#[test]
fn returns_impl_termination_success() -> impl Termination {
    allure.description("Verifies opaque impl Termination return values can pass.");
    allure.log_step("return successful opaque termination");
    SampleTermination { succeeds: true }
}

#[allure_test]
#[test]
fn returns_impl_termination_failure() -> impl Termination {
    allure.description("Verifies opaque impl Termination return values can fail Cargo and Allure.");
    allure.log_step("return failing opaque termination");
    SampleTermination { succeeds: false }
}
