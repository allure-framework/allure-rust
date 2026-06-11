//! Framework-neutral high-level Allure runtime facade.

use std::{
    cell::RefCell,
    fs,
    future::Future,
    panic::{self, AssertUnwindSafe},
    path::Path,
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering},
        OnceLock,
    },
    task::{Context, Poll},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    config::{
        apply_common_runtime_labels, apply_config_labels, apply_synthetic_suite_labels,
        title_path as config_title_path,
    },
    current_owner, error_classifier,
    http_exchange::{HttpExchange, HTTP_EXCHANGE_ATTACHMENT_NAME},
    lifecycle::{AllureLifecycle, AllureRuntime, StartTestCaseParams},
    model::{GlobalAttachment, GlobalError, Globals, Label, ParameterMode, Status, StatusDetails},
    writer::{FileSystemResultsWriter, PLAYWRIGHT_TRACE_ATTACHMENT_MIME},
};

static ALLURE: OnceLock<AllureFacade> = OnceLock::new();
static FACADE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static CURRENT_ALLURE: RefCell<Option<AllureFacade>> = const { RefCell::new(None) };
    static LAST_ASSERTION_FAILURE: RefCell<Option<StatusDetails>> = const { RefCell::new(None) };
}

/// Returns the process-wide default facade.
pub fn allure() -> &'static AllureFacade {
    ALLURE.get_or_init(AllureFacade::default)
}

/// Guard that restores the previous thread-bound facade when dropped.
pub struct CurrentAllureGuard {
    previous: Option<AllureFacade>,
}

/// Binds an Allure facade to the current thread.
pub fn push_current_allure(allure: &AllureFacade) -> CurrentAllureGuard {
    let previous = CURRENT_ALLURE.with(|current| current.replace(Some(allure.clone())));
    CurrentAllureGuard { previous }
}

/// Returns the facade currently bound to this thread.
pub fn current_allure() -> Option<AllureFacade> {
    CURRENT_ALLURE.with(|current| current.borrow().clone())
}

/// Clears pending assertion failure details captured by assertion logging.
pub fn clear_last_assertion_failure() {
    LAST_ASSERTION_FAILURE.with(|failure| {
        failure.replace(None);
    });
}

/// Returns status details for a failure message, reusing captured assertion details when present.
pub fn status_details_for_message(message: String) -> StatusDetails {
    LAST_ASSERTION_FAILURE
        .with(|failure| failure.take())
        .filter(|details| details.message.as_deref() == Some(message.as_str()))
        .unwrap_or(StatusDetails {
            message: Some(message),
            trace: None,
            actual: None,
            expected: None,
        })
}

/// Records a passed assertion as a log step on the current facade.
pub fn record_assertion_pass(name: impl Into<String>) {
    if let Some(allure) = current_allure() {
        allure.log_step(name);
    }
}

#[track_caller]
/// Records a failed assertion and panics with the original assertion message.
pub fn fail_assertion(
    name: impl Into<String>,
    message: String,
    actual: Option<String>,
    expected: Option<String>,
) -> ! {
    let details = StatusDetails {
        message: Some(message.clone()),
        trace: None,
        actual,
        expected,
    };
    LAST_ASSERTION_FAILURE.with(|failure| {
        failure.replace(Some(details.clone()));
    });
    if let Some(allure) = current_allure() {
        let guard = allure.start_step_scope(name);
        let guard = guard.with_status(Status::Failed, Some(details));
        drop(guard);
    }
    panic!("{message}");
}

fn active_allure() -> AllureFacade {
    current_allure().unwrap_or_default()
}

fn facade_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default();
    let counter = FACADE_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{millis}-{counter}")
}

fn write_globals(globals: Globals) -> std::io::Result<()> {
    FileSystemResultsWriter::from_env()?
        .write_globals_typed(&globals)
        .map(|_| ())
}

/// Options used by the macro-free `test` runtime wrappers.
#[derive(Clone, Debug)]
pub struct TestOptions {
    params: StartTestCaseParams,
    manifest_dir: Option<String>,
    module_path: Option<String>,
    title_path: Option<Vec<String>>,
    framework: Option<String>,
    synthetic_suite_labels: bool,
    panic_status: Option<Status>,
}

impl TestOptions {
    /// Creates options for a test with the given display name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            params: StartTestCaseParams::new(name),
            manifest_dir: None,
            module_path: None,
            title_path: None,
            framework: Some("cargo-test".to_string()),
            synthetic_suite_labels: true,
            panic_status: None,
        }
    }

    /// Infers options from the current cargo-test thread name.
    pub fn inferred() -> Self {
        let full_name = current_test_full_name();
        let name = test_name_from_full_name(&full_name);
        Self::new(name).with_full_name(full_name)
    }

    /// Creates options from low-level lifecycle start parameters.
    pub fn from_params(params: StartTestCaseParams) -> Self {
        Self {
            params,
            manifest_dir: None,
            module_path: None,
            title_path: None,
            framework: Some("cargo-test".to_string()),
            synthetic_suite_labels: true,
            panic_status: None,
        }
    }

    /// Sets the test full name.
    pub fn with_full_name(mut self, full_name: impl Into<String>) -> Self {
        self.params.full_name = Some(full_name.into());
        self
    }

    /// Sets an explicit history identifier.
    pub fn with_history_id(mut self, history_id: impl Into<String>) -> Self {
        self.params.history_id = Some(history_id.into());
        self
    }

    /// Sets an explicit test case identifier.
    pub fn with_test_case_id(mut self, test_case_id: impl Into<String>) -> Self {
        self.params.test_case_id = Some(test_case_id.into());
        self
    }

    /// Sets the markdown description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.params.description = Some(description.into());
        self
    }

    /// Sets the HTML description.
    pub fn with_description_html(mut self, description: impl Into<String>) -> Self {
        self.params.description_html = Some(description.into());
        self
    }

    /// Adds an initial label.
    pub fn with_label(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.labels.push(Label {
            name: name.into(),
            value: value.into(),
        });
        self
    }

    /// Adds an initial Allure ID label.
    pub fn with_allure_id(self, value: impl Into<String>) -> Self {
        self.with_label("ALLURE_ID", value)
    }

    /// Sets the Cargo manifest directory used for package metadata lookup.
    pub fn with_manifest_dir(mut self, manifest_dir: impl Into<String>) -> Self {
        self.manifest_dir = Some(manifest_dir.into());
        self
    }

    /// Sets the Rust module path used for package metadata lookup.
    pub fn with_module_path(mut self, module_path: impl Into<String>) -> Self {
        self.module_path = Some(module_path.into());
        self
    }

    /// Sets the title path.
    pub fn with_title_path<I, V>(mut self, title_path: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<String>,
    {
        self.title_path = Some(title_path.into_iter().map(Into::into).collect());
        self
    }

    /// Sets source-location metadata used for title path and package metadata labels.
    pub fn with_source(
        mut self,
        file: impl AsRef<str>,
        manifest_dir: impl Into<String>,
        module_path: impl Into<String>,
    ) -> Self {
        let manifest_dir = manifest_dir.into();
        self.title_path = Some(config_title_path(file.as_ref(), &manifest_dir));
        self.manifest_dir = Some(manifest_dir);
        self.module_path = Some(module_path.into());
        self
    }

    /// Sets the framework label value.
    pub fn with_framework(mut self, framework: impl Into<String>) -> Self {
        self.framework = Some(framework.into());
        self
    }

    /// Disables the framework label.
    pub fn without_framework(mut self) -> Self {
        self.framework = None;
        self
    }

    /// Disables synthetic suite labels.
    pub fn without_synthetic_suite_labels(mut self) -> Self {
        self.synthetic_suite_labels = false;
        self
    }

    /// Sets the status to use for unclassified panics.
    pub fn with_panic_status(mut self, status: Status) -> Self {
        self.panic_status = Some(status);
        self
    }
}

impl From<StartTestCaseParams> for TestOptions {
    fn from(params: StartTestCaseParams) -> Self {
        Self::from_params(params)
    }
}

/// Runs a synchronous test body with inferred Allure metadata.
pub fn test<R, F>(body: F) -> R
where
    F: FnOnce() -> R,
{
    test_with(TestOptions::inferred(), body)
}

/// Runs a synchronous test body with a custom display name.
pub fn test_named<R, F>(name: impl Into<String>, body: F) -> R
where
    F: FnOnce() -> R,
{
    let full_name = current_test_full_name();
    test_with(TestOptions::new(name).with_full_name(full_name), body)
}

/// Runs a synchronous test body with explicit options.
pub fn test_with<R, F>(options: impl Into<TestOptions>, body: F) -> R
where
    F: FnOnce() -> R,
{
    let runtime_test = start_runtime_test(options.into());
    let _current_allure = push_current_allure(&runtime_test.allure);
    let result = panic::catch_unwind(AssertUnwindSafe(body));
    finish_runtime_test(runtime_test, result)
}

/// Runs an async test future with inferred Allure metadata.
pub async fn test_async<R, F>(future: F) -> R
where
    F: Future<Output = R>,
{
    test_with_async(TestOptions::inferred(), future).await
}

/// Runs an async test future with a custom display name.
pub async fn test_named_async<R, F>(name: impl Into<String>, future: F) -> R
where
    F: Future<Output = R>,
{
    let full_name = current_test_full_name();
    test_with_async(TestOptions::new(name).with_full_name(full_name), future).await
}

/// Runs an async test future with explicit options.
pub async fn test_with_async<R, F>(options: impl Into<TestOptions>, future: F) -> R
where
    F: Future<Output = R>,
{
    let runtime_test = start_runtime_test(options.into());
    let result = RuntimeTestFuture {
        allure: runtime_test.allure.clone(),
        future: Box::pin(future),
    }
    .await;
    finish_runtime_test(runtime_test, result)
}

struct RuntimeTest {
    allure: AllureFacade,
    panic_status: Option<Status>,
}

fn start_runtime_test(options: TestOptions) -> RuntimeTest {
    let writer = FileSystemResultsWriter::from_env().expect("allure writer should be created");
    let runtime = AllureRuntime::new(writer);
    let allure = AllureFacade::with_lifecycle(runtime.lifecycle());
    let TestOptions {
        params,
        manifest_dir,
        module_path,
        title_path,
        framework,
        synthetic_suite_labels,
        panic_status,
    } = options;
    let full_name = params.full_name.clone();

    allure.start_test_case(params);
    clear_last_assertion_failure();
    apply_common_runtime_labels(&allure);
    if let Some(framework) = framework {
        allure.label("framework", framework);
    }
    if synthetic_suite_labels {
        apply_synthetic_suite_labels(&allure, full_name.as_deref());
    }
    if let Some(title_path) = title_path {
        if let (Some(manifest_dir), Some(module_path)) = (&manifest_dir, &module_path) {
            apply_config_labels(&allure, manifest_dir, module_path, &title_path);
        }
        allure.title_path(title_path);
    } else if let (Some(manifest_dir), Some(module_path)) = (&manifest_dir, &module_path) {
        apply_config_labels(&allure, manifest_dir, module_path, &[]);
    }

    RuntimeTest {
        allure,
        panic_status,
    }
}

fn finish_runtime_test<R>(runtime_test: RuntimeTest, result: std::thread::Result<R>) -> R {
    match result {
        Ok(value) => {
            runtime_test.allure.end_test(Status::Passed, None);
            value
        }
        Err(payload) => {
            let (status, details) = error_classifier::classify_panic(&payload);
            let status = runtime_test.panic_status.unwrap_or(status);
            let message = details.message.clone().unwrap_or_default();
            runtime_test
                .allure
                .end_test(status, Some(status_details_for_message(message)));
            panic::resume_unwind(payload);
        }
    }
}

fn current_test_full_name() -> String {
    std::thread::current()
        .name()
        .map(ToString::to_string)
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| format!("{:?}", std::thread::current().id()))
}

fn test_name_from_full_name(full_name: &str) -> String {
    full_name
        .rsplit("::")
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(full_name)
        .to_string()
}

struct RuntimeTestFuture<F> {
    allure: AllureFacade,
    future: Pin<Box<F>>,
}

impl<F> Future for RuntimeTestFuture<F>
where
    F: Future,
{
    type Output = std::thread::Result<F::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let _current_allure = push_current_allure(&this.allure);
        match panic::catch_unwind(AssertUnwindSafe(|| this.future.as_mut().poll(cx))) {
            Ok(Poll::Ready(output)) => Poll::Ready(Ok(output)),
            Ok(Poll::Pending) => Poll::Pending,
            Err(payload) => Poll::Ready(Err(payload)),
        }
    }
}

/// Sets the markdown description on the current test.
pub fn description(description: impl Into<String>) {
    active_allure().description(description);
}

/// Sets the HTML description on the current test.
pub fn description_html(description: impl Into<String>) {
    active_allure().description_html(description);
}

/// Sets the display name of the current test.
pub fn display_name(name: impl Into<String>) {
    active_allure().display_name(name);
}

/// Adds a label to the current test.
pub fn label(name: impl Into<String>, value: impl Into<String>) {
    active_allure().label(name, value);
}

/// Adds multiple labels to the current test.
pub fn labels<I, K, V>(labels: I)
where
    I: IntoIterator<Item = (K, V)>,
    K: Into<String>,
    V: Into<String>,
{
    active_allure().labels(labels);
}

/// Adds a link to the current test.
pub fn link<U, N, T>(url: U, name: Option<N>, link_type: Option<T>)
where
    U: Into<String>,
    N: Into<String>,
    T: Into<String>,
{
    active_allure().link(url, name.map(Into::into), link_type.map(Into::into));
}

/// Adds multiple links to the current test.
pub fn links<I, U, N, T>(links: I)
where
    I: IntoIterator<Item = (U, Option<N>, Option<T>)>,
    U: Into<String>,
    N: Into<String>,
    T: Into<String>,
{
    active_allure().links(links);
}

/// Adds a parameter to the current test.
pub fn parameter(name: impl Into<String>, value: impl Into<String>) {
    active_allure().parameter(name, value);
}

/// Adds a parameter and controls whether it participates in history identity.
pub fn parameter_excluded(name: impl Into<String>, value: impl Into<String>, excluded: bool) {
    active_allure().parameter_excluded(name, value, excluded);
}

/// Adds a parameter with a display mode.
pub fn parameter_mode(name: impl Into<String>, value: impl Into<String>, mode: ParameterMode) {
    active_allure().parameter_mode(name, value, mode);
}

/// Adds a parameter with identity and display options.
pub fn parameter_with_options(
    name: impl Into<String>,
    value: impl Into<String>,
    excluded: Option<bool>,
    mode: Option<ParameterMode>,
) {
    active_allure().parameter_with_options(name, value, excluded, mode);
}

/// Sets the current test title path.
pub fn set_title_path<I, V>(title_path: I)
where
    I: IntoIterator<Item = V>,
    V: Into<String>,
{
    active_allure().title_path(title_path);
}

/// Sets the current test case identifier.
pub fn test_case_id(value: impl Into<String>) {
    active_allure().test_case_id(value);
}

/// Sets the current test history identifier.
pub fn history_id(value: impl Into<String>) {
    active_allure().history_id(value);
}

/// Adds an attachment as an ordered evidence step.
pub fn attachment(
    name: impl Into<String>,
    content_type: impl Into<String>,
    body: impl AsRef<[u8]>,
) {
    active_allure().attachment(name, content_type, body);
}

/// Adds a file attachment as an ordered evidence step.
pub fn attachment_path(
    name: impl Into<String>,
    content_type: impl Into<String>,
    path: impl AsRef<Path>,
) -> std::io::Result<()> {
    active_allure().attachment_path(name, content_type, path)
}

/// Adds a Playwright trace attachment named `trace.zip`.
pub fn attach_trace(path: impl AsRef<Path>) -> std::io::Result<()> {
    active_allure().attach_trace(path)
}

/// Adds a named Playwright trace attachment.
pub fn attach_trace_named(name: impl Into<String>, path: impl AsRef<Path>) -> std::io::Result<()> {
    active_allure().attach_trace_named(name, path)
}

/// Adds a run-level attachment.
pub fn global_attachment(
    name: impl Into<String>,
    content_type: impl Into<String>,
    body: impl AsRef<[u8]>,
) -> std::io::Result<()> {
    active_allure().global_attachment(name, content_type, body)
}

/// Adds a run-level file attachment.
pub fn global_attachment_path(
    name: impl Into<String>,
    content_type: impl Into<String>,
    path: impl AsRef<Path>,
) -> std::io::Result<()> {
    active_allure().global_attachment_path(name, content_type, path)
}

/// Adds a run-level error.
pub fn global_error(message: impl Into<String>) -> std::io::Result<()> {
    active_allure().global_error(message)
}

/// Adds a run-level error with a trace.
pub fn global_error_with_trace(
    message: impl Into<String>,
    trace: impl Into<String>,
) -> std::io::Result<()> {
    active_allure().global_error_with_trace(message, trace)
}

/// Adds an HTTP exchange as an ordered evidence step.
pub fn http_exchange(exchange: HttpExchange) {
    active_allure().http_exchange(exchange);
}

/// Adds a named HTTP exchange as an ordered evidence step.
pub fn http_exchange_named(name: impl Into<String>, exchange: HttpExchange) {
    active_allure().http_exchange_named(name, exchange);
}

/// Runs a lambda-style step.
pub fn step<T, F>(name: impl Into<String>, body: F) -> T
where
    F: FnOnce() -> T,
{
    active_allure().step(name, body)
}

/// Starts a semantic stage under the current owner.
pub fn stage(name: impl Into<String>) {
    active_allure().stage(name);
}

/// Records a passed log step.
pub fn log_step(name: impl Into<String>) {
    active_allure().log_step(name);
}

/// Records an instant log step with optional status and error details.
pub fn log_step_with<E>(name: impl Into<String>, status: Option<Status>, error: Option<E>)
where
    E: ToString,
{
    active_allure().log_step_with(name, status, error);
}

/// Adds an issue link to the current test.
pub fn issue(name: impl Into<String>, url: impl Into<String>) {
    active_allure().issue(name, url);
}

/// Adds a test-management-system link to the current test.
pub fn tms(name: impl Into<String>, url: impl Into<String>) {
    active_allure().tms(name, url);
}

/// Adds an `epic` label.
pub fn epic(value: impl Into<String>) {
    active_allure().epic(value);
}

/// Adds a `feature` label.
pub fn feature(value: impl Into<String>) {
    active_allure().feature(value);
}

/// Adds a `story` label.
pub fn story(value: impl Into<String>) {
    active_allure().story(value);
}

/// Sets the `suite` label.
pub fn suite(value: impl Into<String>) {
    active_allure().suite(value);
}

/// Sets the `parentSuite` label.
pub fn parent_suite(value: impl Into<String>) {
    active_allure().parent_suite(value);
}

/// Sets the `subSuite` label.
pub fn sub_suite(value: impl Into<String>) {
    active_allure().sub_suite(value);
}

/// Adds an `owner` label.
pub fn owner(value: impl Into<String>) {
    active_allure().owner(value);
}

/// Adds a `severity` label.
pub fn severity(value: impl Into<String>) {
    active_allure().severity(value);
}

/// Adds a `layer` label.
pub fn layer(value: impl Into<String>) {
    active_allure().layer(value);
}

/// Adds a `tag` label.
pub fn tag(value: impl Into<String>) {
    active_allure().tag(value);
}

/// Adds multiple `tag` labels.
pub fn tags<I, V>(tags: I)
where
    I: IntoIterator<Item = V>,
    V: Into<String>,
{
    active_allure().tags(tags);
}

/// Adds the Allure ID label.
pub fn id(value: impl Into<String>) {
    active_allure().id(value);
}

/// Adds the Allure ID label.
pub fn allure_id(value: impl Into<String>) {
    active_allure().allure_id(value);
}

impl Drop for CurrentAllureGuard {
    fn drop(&mut self) {
        CURRENT_ALLURE.with(|current| {
            current.replace(self.previous.take());
        });
    }
}

/// Explicit Allure runtime facade.
#[derive(Clone, Default)]
pub struct AllureFacade {
    lifecycle: Option<AllureLifecycle>,
}

impl AllureFacade {
    /// Creates a facade backed by a lifecycle owner.
    pub fn with_lifecycle(lifecycle: AllureLifecycle) -> Self {
        Self {
            lifecycle: Some(lifecycle),
        }
    }

    /// Replaces the lifecycle owner used by this facade.
    pub fn set_lifecycle(&mut self, lifecycle: AllureLifecycle) {
        self.lifecycle = Some(lifecycle);
    }

    pub(crate) fn lifecycle(&self) -> Option<&AllureLifecycle> {
        self.lifecycle.as_ref()
    }

    /// Starts a test case through the underlying lifecycle.
    pub fn start_test_case(&self, params: impl Into<StartTestCaseParams>) {
        if let Some(l) = &self.lifecycle {
            l.start_test_case(params);
        }
    }

    /// Stops the current test case through the underlying lifecycle.
    pub fn stop_test_case(&self, status: Status, details: Option<StatusDetails>) {
        if let Some(l) = &self.lifecycle {
            l.stop_test_case(status, details);
        }
    }

    /// Starts a test with a display name.
    pub fn start_test(&self, name: impl Into<String>) {
        self.start_test_case(name.into());
    }

    /// Starts a test with a display name and full name.
    pub fn start_test_with_full_name(&self, name: impl Into<String>, full_name: impl Into<String>) {
        self.start_test_case(StartTestCaseParams::new(name).with_full_name(full_name));
    }

    /// Ends the current test with a final status.
    pub fn end_test(&self, status: Status, details: Option<StatusDetails>) {
        self.stop_test_case(status, details);
    }

    /// Sets the markdown description on the current test.
    pub fn description(&self, description: impl Into<String>) {
        if let Some(l) = &self.lifecycle {
            l.update_test_case(|t| t.description = Some(description.into()));
        }
    }

    /// Sets the HTML description on the current test.
    pub fn description_html(&self, description: impl Into<String>) {
        if let Some(l) = &self.lifecycle {
            l.update_test_case(|t| t.description_html = Some(description.into()));
        }
    }

    /// Sets the display name of the current test.
    pub fn display_name(&self, name: impl Into<String>) {
        if let Some(l) = &self.lifecycle {
            l.update_test_case(|t| t.name = name.into());
        }
    }

    /// Adds a label to the current test.
    pub fn label(&self, name: impl Into<String>, value: impl Into<String>) {
        if let Some(l) = &self.lifecycle {
            l.add_label(name, value);
        }
    }

    /// Adds multiple labels to the current test.
    pub fn labels<I, K, V>(&self, labels: I)
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (k, v) in labels {
            self.label(k, v);
        }
    }

    /// Adds a link to the current test.
    pub fn link(&self, url: impl Into<String>, name: Option<String>, link_type: Option<String>) {
        if let Some(l) = &self.lifecycle {
            l.add_link(url, name, link_type);
        }
    }

    /// Adds multiple links to the current test.
    pub fn links<I, U, N, T>(&self, links: I)
    where
        I: IntoIterator<Item = (U, Option<N>, Option<T>)>,
        U: Into<String>,
        N: Into<String>,
        T: Into<String>,
    {
        for (url, name, link_type) in links {
            self.link(url, name.map(Into::into), link_type.map(Into::into));
        }
    }

    /// Adds a parameter to the current test.
    pub fn parameter(&self, name: impl Into<String>, value: impl Into<String>) {
        self.parameter_with_options(name, value, None, None);
    }

    /// Adds a parameter and controls whether it participates in history identity.
    pub fn parameter_excluded(
        &self,
        name: impl Into<String>,
        value: impl Into<String>,
        excluded: bool,
    ) {
        self.parameter_with_options(name, value, Some(excluded), None);
    }

    /// Adds a parameter with a display mode.
    pub fn parameter_mode(
        &self,
        name: impl Into<String>,
        value: impl Into<String>,
        mode: ParameterMode,
    ) {
        self.parameter_with_options(name, value, None, Some(mode));
    }

    /// Adds a parameter with identity and display options.
    pub fn parameter_with_options(
        &self,
        name: impl Into<String>,
        value: impl Into<String>,
        excluded: Option<bool>,
        mode: Option<ParameterMode>,
    ) {
        if let Some(l) = &self.lifecycle {
            l.add_parameter_with_options(name, value, excluded, mode);
        }
    }

    /// Sets the current test title path.
    pub fn title_path<I, V>(&self, title_path: I)
    where
        I: IntoIterator<Item = V>,
        V: Into<String>,
    {
        if let Some(l) = &self.lifecycle {
            let title_path = title_path.into_iter().map(Into::into).collect();
            l.update_test_case(|test| test.title_path = Some(title_path));
        }
    }

    /// Sets the current test case identifier.
    pub fn test_case_id(&self, value: impl Into<String>) {
        if let Some(l) = &self.lifecycle {
            l.set_test_case_id(value);
        }
    }

    /// Sets the current test history identifier.
    pub fn history_id(&self, value: impl Into<String>) {
        if let Some(l) = &self.lifecycle {
            l.set_history_id(value);
        }
    }

    /// Adds an attachment as an ordered evidence step.
    pub fn attachment(
        &self,
        name: impl Into<String>,
        content_type: impl Into<String>,
        body: impl AsRef<[u8]>,
    ) {
        let name = name.into();
        let content_type = content_type.into();
        let body = body.as_ref().to_vec();
        self.step(name.clone(), || {
            current_owner::add_attachment(self, name, content_type, body);
        });
    }

    /// Adds a file attachment as an ordered evidence step.
    pub fn attachment_path(
        &self,
        name: impl Into<String>,
        content_type: impl Into<String>,
        path: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        let bytes = fs::read(path)?;
        self.attachment(name, content_type, bytes);
        Ok(())
    }

    /// Adds a Playwright trace attachment named `trace.zip`.
    pub fn attach_trace(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        self.attach_trace_named("trace.zip", path)
    }

    /// Adds a named Playwright trace attachment.
    pub fn attach_trace_named(
        &self,
        name: impl Into<String>,
        path: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        self.attachment_path(name, PLAYWRIGHT_TRACE_ATTACHMENT_MIME, path)
    }

    /// Adds a run-level attachment.
    pub fn global_attachment(
        &self,
        name: impl Into<String>,
        content_type: impl Into<String>,
        body: impl AsRef<[u8]>,
    ) -> std::io::Result<()> {
        let name = name.into();
        let content_type = content_type.into();
        if let Some(l) = &self.lifecycle {
            return l.add_global_attachment(name, content_type, body.as_ref());
        }

        let writer = FileSystemResultsWriter::from_env()?;
        let (source, _) = writer.write_attachment_auto(
            &facade_id(),
            Some(&name),
            Some(&content_type),
            body.as_ref(),
        )?;
        write_globals(Globals {
            attachments: vec![GlobalAttachment {
                name,
                source,
                content_type,
            }],
            errors: Vec::new(),
        })
    }

    /// Adds a run-level file attachment.
    pub fn global_attachment_path(
        &self,
        name: impl Into<String>,
        content_type: impl Into<String>,
        path: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        let bytes = fs::read(path)?;
        self.global_attachment(name, content_type, bytes)
    }

    /// Adds a run-level error.
    pub fn global_error(&self, message: impl Into<String>) -> std::io::Result<()> {
        let message = message.into();
        if let Some(l) = &self.lifecycle {
            return l.add_global_error(message, None);
        }

        write_globals(Globals {
            attachments: Vec::new(),
            errors: vec![GlobalError {
                message,
                trace: None,
            }],
        })
    }

    /// Adds a run-level error with a trace.
    pub fn global_error_with_trace(
        &self,
        message: impl Into<String>,
        trace: impl Into<String>,
    ) -> std::io::Result<()> {
        let message = message.into();
        let trace = trace.into();
        if let Some(l) = &self.lifecycle {
            return l.add_global_error(message, Some(trace));
        }

        write_globals(Globals {
            attachments: Vec::new(),
            errors: vec![GlobalError {
                message,
                trace: Some(trace),
            }],
        })
    }

    /// Adds an HTTP exchange as an ordered evidence step.
    pub fn http_exchange(&self, exchange: HttpExchange) {
        self.http_exchange_named(HTTP_EXCHANGE_ATTACHMENT_NAME, exchange);
    }

    /// Adds a named HTTP exchange as an ordered evidence step.
    pub fn http_exchange_named(&self, name: impl Into<String>, exchange: HttpExchange) {
        let name = name.into();
        self.step(name.clone(), || {
            current_owner::add_http_exchange_named(self, name, exchange);
        });
    }

    fn start_step_scope(&self, name: impl Into<String>) -> StepScope {
        if let Some(l) = &self.lifecycle {
            l.start_step(name);
            StepScope {
                lifecycle: self.lifecycle.clone(),
                status: Some(Status::Passed),
                details: None,
            }
        } else {
            StepScope {
                lifecycle: None,
                status: None,
                details: None,
            }
        }
    }

    /// Runs a lambda-style step.
    pub fn step<T, F>(&self, name: impl Into<String>, body: F) -> T
    where
        F: FnOnce() -> T,
    {
        let guard = self.start_step_scope(name);
        let outcome = panic::catch_unwind(AssertUnwindSafe(body));
        match outcome {
            Ok(value) => {
                drop(guard);
                value
            }
            Err(payload) => {
                let (status, details) = error_classifier::classify_panic(&payload);
                drop(guard.with_status(status, Some(details)));
                panic::resume_unwind(payload);
            }
        }
    }

    /// Starts a semantic stage under the current owner.
    pub fn stage(&self, name: impl Into<String>) {
        if let Some(l) = &self.lifecycle {
            l.start_stage(name);
        }
    }

    /// Records a passed log step.
    pub fn log_step(&self, name: impl Into<String>) {
        self.log_step_with(name, None, None::<String>);
    }

    /// Records an instant log step with optional status and error details.
    pub fn log_step_with<E>(
        &self,
        name: impl Into<String>,
        status: Option<Status>,
        error: Option<E>,
    ) where
        E: ToString,
    {
        if let Some(l) = &self.lifecycle {
            let timestamp = l.start_step_at(name, None);
            let status = status.unwrap_or(Status::Passed);
            let details = error.map(|error| StatusDetails {
                message: Some(error.to_string()),
                trace: None,
                actual: None,
                expected: None,
            });
            l.stop_step_at(Some(timestamp), status, details);
        }
    }

    /// Adds an issue link to the current test.
    pub fn issue(&self, name: impl Into<String>, url: impl Into<String>) {
        self.link(url.into(), Some(name.into()), Some("issue".to_string()));
    }

    /// Adds a test-management-system link to the current test.
    pub fn tms(&self, name: impl Into<String>, url: impl Into<String>) {
        self.link(url.into(), Some(name.into()), Some("tms".to_string()));
    }

    /// Adds an `epic` label.
    pub fn epic(&self, value: impl Into<String>) {
        self.label("epic", value);
    }
    /// Adds a `feature` label.
    pub fn feature(&self, value: impl Into<String>) {
        self.label("feature", value);
    }
    /// Adds a `story` label.
    pub fn story(&self, value: impl Into<String>) {
        self.label("story", value);
    }
    /// Sets the `suite` label.
    pub fn suite(&self, value: impl Into<String>) {
        self.label("suite", value);
    }
    /// Sets the `parentSuite` label.
    pub fn parent_suite(&self, value: impl Into<String>) {
        self.label("parentSuite", value);
    }
    /// Sets the `subSuite` label.
    pub fn sub_suite(&self, value: impl Into<String>) {
        self.label("subSuite", value);
    }
    /// Adds an `owner` label.
    pub fn owner(&self, value: impl Into<String>) {
        self.label("owner", value);
    }
    /// Adds a `severity` label.
    pub fn severity(&self, value: impl Into<String>) {
        self.label("severity", value);
    }
    /// Adds a `layer` label.
    pub fn layer(&self, value: impl Into<String>) {
        self.label("layer", value);
    }
    /// Adds a `tag` label.
    pub fn tag(&self, value: impl Into<String>) {
        self.label("tag", value);
    }
    /// Adds multiple `tag` labels.
    pub fn tags<I, V>(&self, tags: I)
    where
        I: IntoIterator<Item = V>,
        V: Into<String>,
    {
        for tag in tags {
            self.tag(tag);
        }
    }
    /// Adds the Allure ID label.
    pub fn id(&self, value: impl Into<String>) {
        self.label("ALLURE_ID", value);
    }
    /// Adds the Allure ID label.
    pub fn allure_id(&self, value: impl Into<String>) {
        self.id(value);
    }
}

struct StepScope {
    lifecycle: Option<AllureLifecycle>,
    status: Option<Status>,
    details: Option<StatusDetails>,
}

impl StepScope {
    fn with_status(mut self, status: Status, details: Option<StatusDetails>) -> Self {
        self.status = Some(status);
        self.details = details;
        self
    }
}

impl Drop for StepScope {
    fn drop(&mut self) {
        if let (Some(l), Some(status)) = (&self.lifecycle, self.status.clone()) {
            l.stop_step(status, self.details.clone());
        }
    }
}

#[doc(hidden)]
pub mod __private {
    use super::*;

    pub struct StepScope {
        allure: AllureFacade,
        status: Status,
        details: Option<StatusDetails>,
    }

    pub fn begin_step_scope(allure: AllureFacade, name: impl Into<String>) -> StepScope {
        if let Some(lifecycle) = &allure.lifecycle {
            lifecycle.start_step(name);
        }
        StepScope {
            allure,
            status: Status::Passed,
            details: None,
        }
    }

    impl Drop for StepScope {
        fn drop(&mut self) {
            let status = if std::thread::panicking() && matches!(self.status, Status::Passed) {
                Status::Broken
            } else {
                self.status.clone()
            };
            if let Some(lifecycle) = &self.allure.lifecycle {
                lifecycle.stop_step(status, self.details.take());
            }
        }
    }
}
