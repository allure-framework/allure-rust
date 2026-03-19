use std::{
    cell::RefCell,
    cmp,
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    md5::md5_hex,
    model::{
        Attachment, FixtureResult, Label, Link, Parameter, Stage, Status, StatusDetails,
        StepResult, TestResult, TestResultContainer,
    },
    writer::FileSystemResultsWriter,
};

thread_local! {
    static ACTIVE_TEST_ROOT: RefCell<Option<String>> = const { RefCell::new(None) };
    static ACTIVE_SCOPE_ROOT: RefCell<Option<String>> = const { RefCell::new(None) };
}

static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

fn next_id() -> String {
    format!(
        "{}-{}",
        now_millis(),
        ID_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

fn round_millis(value: f64) -> i64 {
    value.round() as i64
}

fn normalize_times(
    start: Option<i64>,
    stop: Option<i64>,
    duration: Option<f64>,
    fallback_stop: i64,
) -> (Option<i64>, Option<i64>) {
    let rounded_duration = duration.map(round_millis).map(|value| cmp::max(value, 0));

    let (start, stop) = match (start, stop, rounded_duration) {
        (Some(start), Some(stop), _) => (start, cmp::max(stop, start)),
        (Some(start), None, Some(duration)) => (start, start.saturating_add(duration)),
        (None, Some(stop), Some(duration)) => (stop.saturating_sub(duration), stop),
        (Some(start), None, None) => (start, cmp::max(fallback_stop, start)),
        (None, Some(stop), None) => (stop, stop),
        (None, None, Some(duration)) => {
            let stop = fallback_stop;
            (stop.saturating_sub(duration), stop)
        }
        (None, None, None) => (fallback_stop, fallback_stop),
    };

    (Some(start), Some(stop))
}

fn normalize_step_result(step: &mut StepResult, fallback_stop: i64) {
    (step.start, step.stop) = normalize_times(step.start, step.stop, None, fallback_stop);
    for nested in &mut step.steps {
        normalize_step_result(nested, step.stop.unwrap_or(fallback_stop));
    }
}

fn normalize_fixture_result(fixture: &mut FixtureResult, fallback_stop: i64) {
    (fixture.start, fixture.stop) =
        normalize_times(fixture.start, fixture.stop, None, fallback_stop);
    let fixture_stop = fixture.stop.unwrap_or(fallback_stop);
    for step in &mut fixture.steps {
        normalize_step_result(step, fixture_stop);
    }
}

fn normalize_test_result(test: &mut TestResult, fallback_stop: i64) {
    (test.start, test.stop) = normalize_times(test.start, test.stop, None, fallback_stop);
    let test_stop = test.stop.unwrap_or(fallback_stop);
    for step in &mut test.steps {
        normalize_step_result(step, test_stop);
    }
}

fn normalize_container_times(container: &mut TestResultContainer, fallback_stop: i64) {
    (container.start, container.stop) =
        normalize_times(container.start, container.stop, None, fallback_stop);
    let container_stop = container.stop.unwrap_or(fallback_stop);
    for fixture in &mut container.befores {
        normalize_fixture_result(fixture, container_stop);
    }
    for fixture in &mut container.afters {
        normalize_fixture_result(fixture, container_stop);
    }
}

fn derive_test_case_id(test: &TestResult) -> Option<String> {
    test.test_case_id
        .clone()
        .or_else(|| test.full_name.clone().map(|full_name| md5_hex(&full_name)))
}

fn derive_history_id(test: &TestResult) -> Option<String> {
    let base = test
        .test_case_id
        .as_ref()
        .or(test.full_name.as_ref())
        .or(Some(&test.name))?;

    let mut parameters = test
        .parameters
        .iter()
        .filter(|parameter| parameter.excluded != Some(true))
        .map(|parameter| format!("{}:{}", parameter.name, parameter.value))
        .collect::<Vec<_>>();
    parameters.sort();
    let parameter_hash = md5_hex(&parameters.join(","));

    Some(md5_hex(&format!("{base}:{parameter_hash}")))
}

#[derive(Clone)]
pub struct AllureRuntime {
    writer: Arc<FileSystemResultsWriter>,
}

impl AllureRuntime {
    pub fn new(writer: FileSystemResultsWriter) -> Self {
        Self {
            writer: Arc::new(writer),
        }
    }

    pub fn lifecycle(&self) -> AllureLifecycle {
        AllureLifecycle {
            writer: Arc::clone(&self.writer),
            state: Arc::new(Mutex::new(LifecycleState::default())),
        }
    }
}

#[derive(Clone)]
pub struct AllureLifecycle {
    writer: Arc<FileSystemResultsWriter>,
    state: Arc<Mutex<LifecycleState>>,
}

#[derive(Debug, Clone, Default)]
pub struct StartTestCaseParams {
    pub uuid: Option<String>,
    pub name: String,
    pub full_name: Option<String>,
    pub history_id: Option<String>,
    pub test_case_id: Option<String>,
    pub description: Option<String>,
    pub description_html: Option<String>,
    pub status: Option<Status>,
    pub status_details: Option<StatusDetails>,
    pub stage: Option<Stage>,
    pub labels: Vec<Label>,
    pub links: Vec<Link>,
    pub parameters: Vec<Parameter>,
    pub steps: Vec<StepResult>,
    pub attachments: Vec<Attachment>,
    pub title_path: Option<Vec<String>>,
    pub start: Option<i64>,
    pub stop: Option<i64>,
}

impl StartTestCaseParams {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn with_full_name(mut self, full_name: impl Into<String>) -> Self {
        self.full_name = Some(full_name.into());
        self
    }
}

impl From<String> for StartTestCaseParams {
    fn from(name: String) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }
}

impl From<&str> for StartTestCaseParams {
    fn from(name: &str) -> Self {
        Self::from(name.to_string())
    }
}

#[derive(Default)]
struct LifecycleState {
    tests: HashMap<String, TestState>,
    scopes: HashMap<String, ScopeState>,
}

struct TestState {
    test: TestResult,
    step_stack: Vec<StepResult>,
    linked_scopes: Vec<String>,
}

struct ScopeState {
    container: TestResultContainer,
    running_fixture: Option<RunningFixture>,
}

struct RunningFixture {
    kind: FixtureKind,
    fixture: FixtureResult,
    step_stack: Vec<StepResult>,
}

enum FixtureKind {
    Before,
    After,
}

impl AllureLifecycle {
    pub fn start_test_case(&self, params: impl Into<StartTestCaseParams>) {
        let params = params.into();
        let name = params.name;
        let uuid = params.uuid.unwrap_or_else(next_id);
        let full_name = params.full_name.or_else(|| Some(name.clone()));

        let mut lock = self.state.lock().expect("poisoned allure lifecycle mutex");
        lock.tests.insert(
            uuid.clone(),
            TestState {
                test: TestResult {
                    uuid: uuid.clone(),
                    name,
                    full_name,
                    history_id: params.history_id,
                    test_case_id: params.test_case_id,
                    description: params.description,
                    description_html: params.description_html,
                    status: params.status,
                    status_details: params.status_details,
                    stage: params.stage.or(Some(Stage::Running)),
                    labels: params.labels,
                    links: params.links,
                    parameters: params.parameters,
                    steps: params.steps,
                    attachments: params.attachments,
                    title_path: params.title_path,
                    start: params.start.or_else(|| Some(now_millis())),
                    stop: params.stop,
                },
                step_stack: Vec::new(),
                linked_scopes: Vec::new(),
            },
        );
        ACTIVE_TEST_ROOT.with(|cell| *cell.borrow_mut() = Some(uuid));
    }

    pub fn current_test_uuid(&self) -> Option<String> {
        ACTIVE_TEST_ROOT.with(|cell| cell.borrow().clone())
    }

    pub fn stop_test_case(&self, status: Status, details: Option<StatusDetails>) {
        let Some(test_uuid) = ACTIVE_TEST_ROOT.with(|cell| cell.borrow().clone()) else {
            return;
        };

        let mut lock = self.state.lock().expect("poisoned allure lifecycle mutex");
        if let Some(mut state) = lock.tests.remove(&test_uuid) {
            finalize_steps(&mut state.step_stack, &mut state.test.steps);
            merge_before_scope_metadata(&lock, &mut state.test, &state.linked_scopes);

            state.test.status = Some(status);
            state.test.status_details = details;
            state.test.stage = Some(Stage::Finished);
            let fallback_stop = now_millis();
            state.test.test_case_id = derive_test_case_id(&state.test);
            state.test.history_id = derive_history_id(&state.test);
            normalize_test_result(&mut state.test, fallback_stop);
            let _ = self.writer.write_result(&state.test);
        }

        ACTIVE_TEST_ROOT.with(|cell| {
            if cell.borrow().as_deref() == Some(test_uuid.as_str()) {
                *cell.borrow_mut() = None;
            }
        });
    }

    pub fn update_test_case<F>(&self, update: F)
    where
        F: FnOnce(&mut TestResult),
    {
        let Some(test_uuid) = ACTIVE_TEST_ROOT.with(|cell| cell.borrow().clone()) else {
            return;
        };
        let mut lock = self.state.lock().expect("poisoned allure lifecycle mutex");
        if let Some(state) = lock.tests.get_mut(&test_uuid) {
            update(&mut state.test);
        }
    }

    pub fn set_test_case_id(&self, test_case_id: impl Into<String>) {
        let test_case_id = test_case_id.into();
        self.update_test_case(|test| test.test_case_id = Some(test_case_id));
    }

    pub fn add_label(&self, name: impl Into<String>, value: impl Into<String>) {
        let name = name.into();
        let value = value.into();
        self.update_test_case(|test| {
            if matches!(name.as_str(), "parentSuite" | "suite" | "subSuite") {
                test.labels.retain(|label| label.name != name);
            }
            test.labels.push(Label {
                name: name.clone(),
                value: value.clone(),
            });
        });
    }

    pub fn add_link(
        &self,
        url: impl Into<String>,
        name: Option<String>,
        link_type: Option<String>,
    ) {
        let url = url.into();
        self.update_test_case(|test| {
            test.links.push(Link {
                name,
                url,
                link_type,
            })
        });
    }

    pub fn add_parameter(&self, name: impl Into<String>, value: impl Into<String>) {
        let name = name.into();
        let value = value.into();
        self.update_test_case(|test| {
            test.parameters.push(Parameter {
                name,
                value,
                excluded: None,
                mode: None,
            })
        });
    }

    pub fn start_scope(&self, name: Option<String>) -> String {
        let uuid = next_id();
        let mut lock = self.state.lock().expect("poisoned allure lifecycle mutex");
        lock.scopes.insert(
            uuid.clone(),
            ScopeState {
                container: TestResultContainer {
                    uuid: uuid.clone(),
                    name,
                    start: Some(now_millis()),
                    ..Default::default()
                },
                running_fixture: None,
            },
        );
        uuid
    }

    pub fn link_scope_to_test(&self, scope_uuid: &str, test_uuid: &str) {
        let mut lock = self.state.lock().expect("poisoned allure lifecycle mutex");
        let has_scope = lock.scopes.contains_key(scope_uuid);
        let has_test = lock.tests.contains_key(test_uuid);
        if !(has_scope && has_test) {
            return;
        }

        if let Some(scope) = lock.scopes.get_mut(scope_uuid) {
            if !scope
                .container
                .children
                .iter()
                .any(|child| child == test_uuid)
            {
                scope.container.children.push(test_uuid.to_string());
            }
        }
        if let Some(test) = lock.tests.get_mut(test_uuid) {
            if !test.linked_scopes.iter().any(|scope| scope == scope_uuid) {
                test.linked_scopes.push(scope_uuid.to_string());
            }
        }
    }

    pub fn stop_scope(&self, scope_uuid: &str) {
        let mut lock = self.state.lock().expect("poisoned allure lifecycle mutex");
        if let Some(scope) = lock.scopes.get_mut(scope_uuid) {
            finish_running_fixture(scope);
            normalize_container_times(&mut scope.container, now_millis());
        }
        ACTIVE_SCOPE_ROOT.with(|cell| {
            if cell.borrow().as_deref() == Some(scope_uuid) {
                *cell.borrow_mut() = None;
            }
        });
    }

    pub fn write_scope(&self, scope_uuid: &str) {
        let mut lock = self.state.lock().expect("poisoned allure lifecycle mutex");
        if let Some(scope) = lock.scopes.remove(scope_uuid) {
            let _ = self.writer.write_container(&scope.container);
        }
    }

    pub fn start_before_fixture(&self, scope_uuid: &str, name: impl Into<String>) {
        self.start_fixture(scope_uuid, name.into(), FixtureKind::Before);
    }

    pub fn stop_before_fixture(
        &self,
        scope_uuid: &str,
        status: Status,
        details: Option<StatusDetails>,
    ) {
        self.stop_fixture(scope_uuid, FixtureKind::Before, status, details);
    }

    pub fn start_after_fixture(&self, scope_uuid: &str, name: impl Into<String>) {
        self.start_fixture(scope_uuid, name.into(), FixtureKind::After);
    }

    pub fn stop_after_fixture(
        &self,
        scope_uuid: &str,
        status: Status,
        details: Option<StatusDetails>,
    ) {
        self.stop_fixture(scope_uuid, FixtureKind::After, status, details);
    }

    pub fn add_attachment(
        &self,
        name: impl Into<String>,
        content_type: impl Into<String>,
        bytes: &[u8],
    ) {
        let name = name.into();
        let content_type = content_type.into();
        let id = next_id();
        if let Ok((source, _)) =
            self.writer
                .write_attachment_auto(&id, Some(&name), Some(&content_type), bytes)
        {
            let attachment = Attachment {
                name,
                source,
                content_type,
            };
            let mut lock = self.state.lock().expect("poisoned allure lifecycle mutex");
            if let Some(scope_uuid) = ACTIVE_SCOPE_ROOT.with(|cell| cell.borrow().clone()) {
                if let Some(scope) = lock.scopes.get_mut(&scope_uuid) {
                    if let Some(fixture) = scope.running_fixture.as_mut() {
                        if let Some(step) = fixture.step_stack.last_mut() {
                            step.attachments.push(attachment);
                        } else {
                            fixture.fixture.attachments.push(attachment);
                        }
                        return;
                    }
                }
            }

            if let Some(test_uuid) = ACTIVE_TEST_ROOT.with(|cell| cell.borrow().clone()) {
                if let Some(test_state) = lock.tests.get_mut(&test_uuid) {
                    if let Some(step) = test_state.step_stack.last_mut() {
                        step.attachments.push(attachment);
                    } else {
                        test_state.test.attachments.push(attachment);
                    }
                }
            }
        }
    }

    pub fn start_step(&self, name: impl Into<String>) {
        self.start_step_at(name, None);
    }

    pub fn start_step_at(&self, name: impl Into<String>, timestamp: Option<i64>) -> i64 {
        let timestamp = timestamp.unwrap_or_else(now_millis);
        let step = StepResult {
            name: name.into(),
            stage: Some(Stage::Running),
            start: Some(timestamp),
            ..Default::default()
        };
        let mut lock = self.state.lock().expect("poisoned allure lifecycle mutex");

        if let Some(scope_uuid) = ACTIVE_SCOPE_ROOT.with(|cell| cell.borrow().clone()) {
            if let Some(scope) = lock.scopes.get_mut(&scope_uuid) {
                if let Some(fixture) = scope.running_fixture.as_mut() {
                    fixture.step_stack.push(step);
                    return timestamp;
                }
            }
        }

        if let Some(test_uuid) = ACTIVE_TEST_ROOT.with(|cell| cell.borrow().clone()) {
            if let Some(test_state) = lock.tests.get_mut(&test_uuid) {
                test_state.step_stack.push(step);
            }
        }

        timestamp
    }

    pub fn stop_step(&self, status: Status, details: Option<StatusDetails>) {
        self.stop_step_at(None, status, details);
    }

    pub fn stop_step_at(
        &self,
        timestamp: Option<i64>,
        status: Status,
        details: Option<StatusDetails>,
    ) {
        let mut lock = self.state.lock().expect("poisoned allure lifecycle mutex");

        if let Some(scope_uuid) = ACTIVE_SCOPE_ROOT.with(|cell| cell.borrow().clone()) {
            if let Some(scope) = lock.scopes.get_mut(&scope_uuid) {
                if let Some(fixture) = scope.running_fixture.as_mut() {
                    stop_one_step(
                        &mut fixture.step_stack,
                        &mut fixture.fixture.steps,
                        timestamp,
                        status,
                        details,
                    );
                    return;
                }
            }
        }

        if let Some(test_uuid) = ACTIVE_TEST_ROOT.with(|cell| cell.borrow().clone()) {
            if let Some(test_state) = lock.tests.get_mut(&test_uuid) {
                stop_one_step(
                    &mut test_state.step_stack,
                    &mut test_state.test.steps,
                    timestamp,
                    status,
                    details,
                );
            }
        }
    }

    pub fn set_current_step_display_name(&self, name: impl Into<String>) {
        let name = name.into();
        self.update_current_step(
            move |step| step.name = name,
            "attempted to rename current step, but no step is active",
        );
    }

    pub fn add_current_step_parameter(&self, name: impl Into<String>, value: impl Into<String>) {
        let parameter = Parameter {
            name: name.into(),
            value: value.into(),
            excluded: None,
            mode: None,
        };
        self.update_current_step(
            move |step| step.parameters.push(parameter),
            "attempted to add a parameter to the current step, but no step is active",
        );
    }

    fn start_fixture(&self, scope_uuid: &str, name: String, kind: FixtureKind) {
        let mut lock = self.state.lock().expect("poisoned allure lifecycle mutex");
        if let Some(scope) = lock.scopes.get_mut(scope_uuid) {
            finish_running_fixture(scope);
            scope.running_fixture = Some(RunningFixture {
                kind,
                fixture: FixtureResult {
                    name,
                    stage: Some(Stage::Running),
                    start: Some(now_millis()),
                    ..Default::default()
                },
                step_stack: Vec::new(),
            });
            ACTIVE_SCOPE_ROOT.with(|cell| *cell.borrow_mut() = Some(scope_uuid.to_string()));
        }
    }

    fn update_current_step<F>(&self, update: F, missing_step_message: &str)
    where
        F: FnOnce(&mut StepResult),
    {
        let mut lock = self.state.lock().expect("poisoned allure lifecycle mutex");

        if let Some(scope_uuid) = ACTIVE_SCOPE_ROOT.with(|cell| cell.borrow().clone()) {
            if let Some(scope) = lock.scopes.get_mut(&scope_uuid) {
                if let Some(fixture) = scope.running_fixture.as_mut() {
                    if let Some(step) = fixture.step_stack.last_mut() {
                        update(step);
                        return;
                    }
                }
            }
        }

        if let Some(test_uuid) = ACTIVE_TEST_ROOT.with(|cell| cell.borrow().clone()) {
            if let Some(test_state) = lock.tests.get_mut(&test_uuid) {
                if let Some(step) = test_state.step_stack.last_mut() {
                    update(step);
                    return;
                }
            }
        }

        eprintln!("[allure-rust] {missing_step_message}");
    }

    fn stop_fixture(
        &self,
        scope_uuid: &str,
        expected_kind: FixtureKind,
        status: Status,
        details: Option<StatusDetails>,
    ) {
        let mut lock = self.state.lock().expect("poisoned allure lifecycle mutex");
        if let Some(scope) = lock.scopes.get_mut(scope_uuid) {
            if let Some(mut fixture) = scope.running_fixture.take() {
                if !matches!(
                    (&fixture.kind, &expected_kind),
                    (FixtureKind::Before, FixtureKind::Before)
                        | (FixtureKind::After, FixtureKind::After)
                ) {
                    scope.running_fixture = Some(fixture);
                    return;
                }

                finalize_steps(&mut fixture.step_stack, &mut fixture.fixture.steps);
                fixture.fixture.status = Some(status);
                fixture.fixture.status_details = details;
                fixture.fixture.stage = Some(Stage::Finished);
                normalize_fixture_result(&mut fixture.fixture, now_millis());
                match fixture.kind {
                    FixtureKind::Before => scope.container.befores.push(fixture.fixture),
                    FixtureKind::After => scope.container.afters.push(fixture.fixture),
                }
            }
        }
        ACTIVE_SCOPE_ROOT.with(|cell| {
            if cell.borrow().as_deref() == Some(scope_uuid) {
                *cell.borrow_mut() = None;
            }
        });
    }
}

fn stop_one_step(
    stack: &mut Vec<StepResult>,
    root_steps: &mut Vec<StepResult>,
    timestamp: Option<i64>,
    status: Status,
    details: Option<StatusDetails>,
) {
    if let Some(mut step) = stack.pop() {
        step.status.get_or_insert(status);
        if step.status_details.is_none() {
            step.status_details = details;
        }
        step.stage = Some(Stage::Finished);
        normalize_step_result(&mut step, timestamp.unwrap_or_else(now_millis));
        if let Some(stop) = timestamp {
            step.stop = Some(stop);
            if step.start.is_none() {
                step.start = Some(stop);
            }
        }
        if let Some(parent) = stack.last_mut() {
            parent.steps.push(step);
        } else {
            root_steps.push(step);
        }
    }
}

fn finalize_steps(stack: &mut Vec<StepResult>, root_steps: &mut Vec<StepResult>) {
    while let Some(mut step) = stack.pop() {
        step.status.get_or_insert(Status::Broken);
        step.stage = Some(Stage::Finished);
        normalize_step_result(&mut step, now_millis());
        if let Some(parent) = stack.last_mut() {
            parent.steps.push(step);
        } else {
            root_steps.push(step);
        }
    }
}

fn finish_running_fixture(scope: &mut ScopeState) {
    if let Some(mut fixture) = scope.running_fixture.take() {
        finalize_steps(&mut fixture.step_stack, &mut fixture.fixture.steps);
        fixture.fixture.status.get_or_insert(Status::Broken);
        fixture.fixture.stage = Some(Stage::Finished);
        normalize_fixture_result(&mut fixture.fixture, now_millis());
        match fixture.kind {
            FixtureKind::Before => scope.container.befores.push(fixture.fixture),
            FixtureKind::After => scope.container.afters.push(fixture.fixture),
        }
    }
}

fn merge_before_scope_metadata(
    lock: &LifecycleState,
    test: &mut TestResult,
    linked_scopes: &[String],
) {
    for scope_uuid in linked_scopes {
        if let Some(scope) = lock.scopes.get(scope_uuid) {
            for link in &scope.container.links {
                test.links.push(link.clone());
            }
            for fixture in &scope.container.befores {
                for parameter in &fixture.parameters {
                    test.parameters.push(parameter.clone());
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod lifecycle_tests;
