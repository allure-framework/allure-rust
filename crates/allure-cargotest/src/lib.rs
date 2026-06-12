//! `cargo test` integration helpers for writing Allure results.
//!
//! The primary entry points are the `#[allure_test]`, `#[step]`, and `#[log_asserts]` macros.
//! `CargoTestReporter` is available for manual integration scenarios.

#![deny(missing_docs)]

extern crate self as allure_cargotest;

pub use allure_rust_commons::{Status, StatusDetails};
/// Attribute procedural macros must live in a `proc-macro` crate.
///
/// This crate re-exports `#[allure_test]` and `#[step]` so consumers only depend on
/// `allure-cargotest` and do not need to import the macro crate directly.
pub use allure_test_macros::{allure_test, log_asserts, step};

use std::{
    any::Any,
    future::Future,
    panic::{catch_unwind, AssertUnwindSafe},
    path::Path,
    pin::Pin,
    task::{Context, Poll},
};

mod testplan;

pub use testplan::{TestPlan, TestPlanEntry};

use allure_rust_commons::{
    apply_common_runtime_labels, apply_config_labels, apply_synthetic_suite_labels, current_allure,
    log_asserts_enabled, push_current_allure, title_path, AllureFacade, AllureRuntime,
    CurrentAllureGuard, FileSystemResultsWriter,
};

#[doc(hidden)]
pub mod __private {
    use super::{
        apply_config_labels as common_apply_config_labels, catch_unwind,
        current_allure as common_current_allure, log_asserts_enabled as common_log_asserts_enabled,
        push_current_allure as common_push_current_allure, title_path as common_title_path,
        AllureFacade, AssertUnwindSafe, CargoTestReporter, Context, CurrentAllureGuard,
        FileSystemResultsWriter, Future, Pin, Poll, ReporterError, StatusDetails, TestPlan,
    };

    pub use allure_rust_commons::{test_with, test_with_async, TestOptions};

    pub fn push_current_allure(allure: &AllureFacade) -> CurrentAllureGuard {
        common_push_current_allure(allure)
    }

    pub fn current_allure() -> Option<AllureFacade> {
        common_current_allure()
    }

    pub fn new_reporter() -> Result<CargoTestReporter, ReporterError> {
        let writer = FileSystemResultsWriter::from_env()?;
        Ok(CargoTestReporter::from_writer(writer))
    }

    pub fn title_path(file: &str, manifest_dir: &str) -> Vec<String> {
        common_title_path(file, manifest_dir)
    }

    pub fn apply_config_labels(
        allure: &AllureFacade,
        manifest_dir: &str,
        module_path: &str,
        title_path: &[String],
    ) {
        common_apply_config_labels(allure, manifest_dir, module_path, title_path);
    }

    pub fn log_asserts_enabled(manifest_dir: &str) -> bool {
        common_log_asserts_enabled(manifest_dir)
    }

    pub fn is_selected(
        full_name: Option<&str>,
        allure_id: Option<&str>,
        tags: Option<&[&str]>,
    ) -> bool {
        match TestPlan::from_env() {
            Some(plan) => plan.is_selected(full_name, allure_id, tags),
            None => true,
        }
    }

    pub fn clear_last_assertion_failure() {
        allure_rust_commons::clear_last_assertion_failure();
    }

    pub fn status_details_for_message(message: String) -> StatusDetails {
        allure_rust_commons::status_details_for_message(message)
    }

    pub fn record_assertion_pass(name: impl Into<String>) {
        allure_rust_commons::record_assertion_pass(name);
    }

    #[track_caller]
    pub fn fail_assertion(
        name: impl Into<String>,
        message: String,
        actual: Option<String>,
        expected: Option<String>,
    ) -> ! {
        allure_rust_commons::fail_assertion(name, message, actual, expected);
    }

    pub fn begin_step_scope(
        allure: AllureFacade,
        name: impl Into<String>,
    ) -> allure_rust_commons::facade::__private::StepScope {
        allure_rust_commons::facade::__private::begin_step_scope(allure, name)
    }

    pub async fn catch_unwind_async<F>(future: F) -> std::thread::Result<F::Output>
    where
        F: Future,
    {
        CatchUnwindFuture {
            future: Box::pin(future),
        }
        .await
    }

    pub(crate) async fn run_with_current_allure<F>(
        allure: AllureFacade,
        future: F,
    ) -> std::thread::Result<F::Output>
    where
        F: Future,
    {
        CurrentAllureFuture {
            allure,
            future: Box::pin(future),
        }
        .await
    }

    struct CatchUnwindFuture<F> {
        future: Pin<Box<F>>,
    }

    impl<F> Future for CatchUnwindFuture<F>
    where
        F: Future,
    {
        type Output = std::thread::Result<F::Output>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.get_mut();
            match catch_unwind(AssertUnwindSafe(|| this.future.as_mut().poll(cx))) {
                Ok(Poll::Ready(output)) => Poll::Ready(Ok(output)),
                Ok(Poll::Pending) => Poll::Pending,
                Err(payload) => Poll::Ready(Err(payload)),
            }
        }
    }

    struct CurrentAllureFuture<F> {
        allure: AllureFacade,
        future: Pin<Box<F>>,
    }

    impl<F> Future for CurrentAllureFuture<F>
    where
        F: Future,
    {
        type Output = std::thread::Result<F::Output>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.get_mut();
            let _current_allure = push_current_allure(&this.allure);
            match catch_unwind(AssertUnwindSafe(|| this.future.as_mut().poll(cx))) {
                Ok(Poll::Ready(output)) => Poll::Ready(Ok(output)),
                Ok(Poll::Pending) => Poll::Pending,
                Err(payload) => Poll::Ready(Err(payload)),
            }
        }
    }
}

/// Error returned by the cargo-test reporter.
#[derive(Debug)]
pub enum ReporterError {
    /// Filesystem I/O failure.
    Io(std::io::Error),
}

impl std::fmt::Display for ReporterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {err}"),
        }
    }
}

impl std::error::Error for ReporterError {}

impl From<std::io::Error> for ReporterError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// Manual reporter for integrating Allure with cargo-test style execution.
#[derive(Clone)]
pub struct CargoTestReporter {
    allure: AllureFacade,
    test_plan: Option<TestPlan>,
}

impl CargoTestReporter {
    /// Creates a reporter that writes results into the given directory.
    pub fn new<P: AsRef<Path>>(results_dir: P) -> Result<Self, ReporterError> {
        let writer = FileSystemResultsWriter::new(results_dir)?;
        Ok(Self::from_writer(writer))
    }

    fn from_writer(writer: FileSystemResultsWriter) -> Self {
        let runtime = AllureRuntime::new(writer);
        Self {
            allure: AllureFacade::with_lifecycle(runtime.lifecycle()),
            test_plan: TestPlan::from_env(),
        }
    }

    /// Returns the underlying Allure facade.
    pub fn allure(&self) -> &AllureFacade {
        &self.allure
    }

    /// Runs a synchronous test with a display name.
    pub fn run_test<F>(&self, name: &str, f: F)
    where
        F: FnOnce(&AllureFacade),
    {
        self.run_test_with_metadata(name, Some(name), None, None, f);
    }

    /// Runs a synchronous test with explicit metadata used for identity and plan filtering.
    pub fn run_test_with_metadata<F>(
        &self,
        test_name: &str,
        full_name: Option<&str>,
        allure_id: Option<&str>,
        tags: Option<&[&str]>,
        f: F,
    ) where
        F: FnOnce(&AllureFacade),
    {
        if !self.is_selected(test_name, full_name, allure_id, tags) {
            return;
        }

        if let Some(full_name) = full_name {
            self.allure.start_test_with_full_name(test_name, full_name);
        } else {
            self.allure.start_test(test_name);
        }
        __private::clear_last_assertion_failure();
        apply_common_runtime_labels(&self.allure);
        self.allure.label("framework", "cargo-test");
        apply_synthetic_suite_labels(&self.allure, full_name);
        let _current_allure = __private::push_current_allure(&self.allure);
        let result = catch_unwind(AssertUnwindSafe(|| f(&self.allure)));
        match result {
            Ok(_) => self.allure.end_test(Status::Passed, None),
            Err(payload) => {
                let msg = if let Some(msg) = payload.downcast_ref::<&str>() {
                    (*msg).to_string()
                } else if let Some(msg) = payload.downcast_ref::<String>() {
                    msg.clone()
                } else {
                    "panic without string payload".to_string()
                };
                self.allure.end_test(
                    Status::Failed,
                    Some(__private::status_details_for_message(msg)),
                );
                std::panic::resume_unwind(payload);
            }
        }
    }

    /// Runs an async test with explicit metadata used for identity and plan filtering.
    pub async fn run_test_with_metadata_async<F>(
        &self,
        test_name: &str,
        full_name: Option<&str>,
        allure_id: Option<&str>,
        tags: Option<&[&str]>,
        future: F,
    ) where
        F: Future<Output = ()>,
    {
        if !self.is_selected(test_name, full_name, allure_id, tags) {
            return;
        }

        if let Some(full_name) = full_name {
            self.allure.start_test_with_full_name(test_name, full_name);
        } else {
            self.allure.start_test(test_name);
        }
        __private::clear_last_assertion_failure();
        apply_common_runtime_labels(&self.allure);
        self.allure.label("framework", "cargo-test");
        apply_synthetic_suite_labels(&self.allure, full_name);
        let result = __private::run_with_current_allure(self.allure.clone(), future).await;
        match result {
            Ok(_) => self.allure.end_test(Status::Passed, None),
            Err(payload) => {
                let msg = panic_message(payload.as_ref());
                self.allure.end_test(
                    Status::Failed,
                    Some(__private::status_details_for_message(msg)),
                );
                std::panic::resume_unwind(payload);
            }
        }
    }

    /// Returns whether a test is selected by the active Allure test plan.
    pub fn is_selected(
        &self,
        _test_name: &str,
        full_name: Option<&str>,
        allure_id: Option<&str>,
        tags: Option<&[&str]>,
    ) -> bool {
        match &self.test_plan {
            Some(plan) => plan.is_selected(full_name, allure_id, tags),
            None => true,
        }
    }

    /// Runs a synchronous test body that returns its final Allure result state.
    pub fn run_test_with_result<F>(&self, name: &str, f: F)
    where
        F: FnOnce(&AllureFacade) -> (Status, Option<StatusDetails>, Option<Box<dyn Any + Send>>),
    {
        self.allure.start_test(name);
        __private::clear_last_assertion_failure();
        apply_common_runtime_labels(&self.allure);
        self.allure.label("framework", "cargo-test");
        let _current_allure = __private::push_current_allure(&self.allure);
        let (status, details, panic_payload) = f(&self.allure);
        self.allure.end_test(status, details);
        if let Some(payload) = panic_payload {
            std::panic::resume_unwind(payload);
        }
    }

    /// Runs an async test body that returns its final Allure result state.
    pub async fn run_test_with_result_async<F>(&self, name: &str, future: F)
    where
        F: Future<Output = (Status, Option<StatusDetails>, Option<Box<dyn Any + Send>>)>,
    {
        self.allure.start_test(name);
        __private::clear_last_assertion_failure();
        apply_common_runtime_labels(&self.allure);
        self.allure.label("framework", "cargo-test");
        let result = __private::run_with_current_allure(self.allure.clone(), future).await;
        match result {
            Ok((status, details, panic_payload)) => {
                self.allure.end_test(status, details);
                if let Some(payload) = panic_payload {
                    std::panic::resume_unwind(payload);
                }
            }
            Err(payload) => {
                let msg = panic_message(payload.as_ref());
                self.allure.end_test(
                    Status::Failed,
                    Some(__private::status_details_for_message(msg)),
                );
                std::panic::resume_unwind(payload);
            }
        }
    }
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(msg) = payload.downcast_ref::<&str>() {
        (*msg).to_string()
    } else if let Some(msg) = payload.downcast_ref::<String>() {
        msg.clone()
    } else {
        "panic without string payload".to_string()
    }
}

#[macro_export]
/// Wraps a block in a manual `CargoTestReporter::run_test` call.
macro_rules! allure_wrap_test {
    ($reporter:expr, $name:expr, $body:block) => {{
        $reporter.run_test($name, |_| $body)
    }};
}
