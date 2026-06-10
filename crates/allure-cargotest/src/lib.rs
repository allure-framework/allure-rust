pub use allure_rust_commons::{Status, StatusDetails};
/// Attribute procedural macros must live in a `proc-macro` crate.
///
/// This crate re-exports `#[allure_test]` and `#[step]` so consumers only depend on
/// `allure-cargotest` and do not need to import the macro crate directly.
pub use allure_test_macros::{allure_test, step};

use std::{
    any::Any,
    cell::RefCell,
    future::Future,
    panic::{catch_unwind, AssertUnwindSafe},
    path::Path,
    pin::Pin,
    task::{Context, Poll},
};

mod labels;
#[cfg(test)]
mod test_utils;
mod testplan;

pub use testplan::{TestPlan, TestPlanEntry};

use allure_rust_commons::{AllureFacade, AllureRuntime, FileSystemResultsWriter};

thread_local! {
    static CURRENT_ALLURE: RefCell<Option<AllureFacade>> = const { RefCell::new(None) };
}

pub mod __private {
    use super::CURRENT_ALLURE;
    use super::{catch_unwind, labels, AllureFacade, AssertUnwindSafe, Context, Future, Pin, Poll};

    pub struct CurrentAllureGuard {
        previous: Option<AllureFacade>,
    }

    pub fn push_current_allure(allure: &AllureFacade) -> CurrentAllureGuard {
        let previous = CURRENT_ALLURE.with(|current| current.replace(Some(allure.clone())));
        CurrentAllureGuard { previous }
    }

    pub fn current_allure() -> Option<AllureFacade> {
        CURRENT_ALLURE.with(|current| current.borrow().clone())
    }

    pub fn title_path(file: &str, manifest_dir: &str) -> Vec<String> {
        relative_file_path(file, manifest_dir)
            .split('/')
            .filter(|part| !part.is_empty())
            .map(ToString::to_string)
            .collect()
    }

    pub fn apply_config_labels(
        allure: &AllureFacade,
        manifest_dir: &str,
        module_path: &str,
        title_path: &[String],
    ) {
        labels::add_config_labels(allure, manifest_dir, module_path, title_path);
    }

    impl Drop for CurrentAllureGuard {
        fn drop(&mut self) {
            CURRENT_ALLURE.with(|current| {
                current.replace(self.previous.take());
            });
        }
    }

    fn relative_file_path(file: &str, manifest_dir: &str) -> String {
        let file = file.replace('\\', "/");
        let manifest_dir = manifest_dir.replace('\\', "/");
        if let Some(relative) = file
            .strip_prefix(&manifest_dir)
            .map(|path| path.trim_start_matches('/'))
        {
            return relative.to_string();
        }

        let Some(package_name) = manifest_dir.rsplit('/').next() else {
            return file;
        };
        let package_segment = format!("/{package_name}/");
        if let Some((_, relative)) = file.split_once(&package_segment) {
            return relative.to_string();
        }
        let package_prefix = format!("{package_name}/");
        if let Some(relative) = file.strip_prefix(&package_prefix) {
            return relative.to_string();
        }

        file
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

#[derive(Debug)]
pub enum ReporterError {
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

#[derive(Clone)]
pub struct CargoTestReporter {
    allure: AllureFacade,
    test_plan: Option<TestPlan>,
}

impl CargoTestReporter {
    pub fn new<P: AsRef<Path>>(results_dir: P) -> Result<Self, ReporterError> {
        let writer = FileSystemResultsWriter::new(results_dir)?;
        let runtime = AllureRuntime::new(writer);
        Ok(Self {
            allure: AllureFacade::with_lifecycle(runtime.lifecycle()),
            test_plan: TestPlan::from_env(),
        })
    }

    pub fn allure(&self) -> &AllureFacade {
        &self.allure
    }

    pub fn run_test<F>(&self, name: &str, f: F)
    where
        F: FnOnce(&AllureFacade),
    {
        self.run_test_with_metadata(name, Some(name), None, None, f);
    }

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
        labels::add_default_and_global_labels(&self.allure);
        labels::add_synthetic_suite_labels(&self.allure, full_name);
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
                    Some(StatusDetails {
                        message: Some(msg),
                        trace: None,
                        actual: None,
                        expected: None,
                    }),
                );
                std::panic::resume_unwind(payload);
            }
        }
    }

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
        labels::add_default_and_global_labels(&self.allure);
        labels::add_synthetic_suite_labels(&self.allure, full_name);
        let result = __private::run_with_current_allure(self.allure.clone(), future).await;
        match result {
            Ok(_) => self.allure.end_test(Status::Passed, None),
            Err(payload) => {
                let msg = panic_message(payload.as_ref());
                self.allure.end_test(
                    Status::Failed,
                    Some(StatusDetails {
                        message: Some(msg),
                        trace: None,
                        actual: None,
                        expected: None,
                    }),
                );
                std::panic::resume_unwind(payload);
            }
        }
    }

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

    pub fn run_test_with_result<F>(&self, name: &str, f: F)
    where
        F: FnOnce(&AllureFacade) -> (Status, Option<StatusDetails>, Option<Box<dyn Any + Send>>),
    {
        self.allure.start_test(name);
        labels::add_default_and_global_labels(&self.allure);
        let _current_allure = __private::push_current_allure(&self.allure);
        let (status, details, panic_payload) = f(&self.allure);
        self.allure.end_test(status, details);
        if let Some(payload) = panic_payload {
            std::panic::resume_unwind(payload);
        }
    }

    pub async fn run_test_with_result_async<F>(&self, name: &str, future: F)
    where
        F: Future<Output = (Status, Option<StatusDetails>, Option<Box<dyn Any + Send>>)>,
    {
        self.allure.start_test(name);
        labels::add_default_and_global_labels(&self.allure);
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
                    Some(StatusDetails {
                        message: Some(msg),
                        trace: None,
                        actual: None,
                        expected: None,
                    }),
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
macro_rules! allure_wrap_test {
    ($reporter:expr, $name:expr, $body:block) => {{
        $reporter.run_test($name, |_| $body)
    }};
}
