//! Serializable Allure result model types.

use serde::Serialize;

/// Serialized Allure test result.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    /// Unique test execution identifier.
    pub uuid: String,
    /// Display name shown in the report.
    pub name: String,
    /// Stable fully qualified test name.
    pub full_name: Option<String>,
    /// Explicit or derived history identifier.
    pub history_id: Option<String>,
    /// Explicit or derived logical test case identifier.
    pub test_case_id: Option<String>,
    /// Markdown description.
    pub description: Option<String>,
    /// HTML description.
    pub description_html: Option<String>,
    /// Final test status.
    pub status: Option<Status>,
    /// Details associated with the final test status.
    pub status_details: Option<StatusDetails>,
    /// Lifecycle stage of the test.
    pub stage: Option<Stage>,
    /// Labels attached to the test.
    pub labels: Vec<Label>,
    /// Links attached to the test.
    pub links: Vec<Link>,
    /// Test-level parameters.
    pub parameters: Vec<Parameter>,
    /// Top-level steps.
    pub steps: Vec<StepResult>,
    /// Test-level attachments.
    pub attachments: Vec<Attachment>,
    /// Hierarchical path that identifies the test in the integration root.
    pub title_path: Option<Vec<String>>,
    /// Start timestamp in milliseconds since the Unix epoch.
    pub start: Option<i64>,
    /// Stop timestamp in milliseconds since the Unix epoch.
    pub stop: Option<i64>,
}

/// Serialized Allure fixture result.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FixtureResult {
    /// Fixture display name.
    pub name: String,
    /// Final fixture status.
    pub status: Option<Status>,
    /// Details associated with the final fixture status.
    pub status_details: Option<StatusDetails>,
    /// Lifecycle stage of the fixture.
    pub stage: Option<Stage>,
    /// Steps recorded inside the fixture.
    pub steps: Vec<StepResult>,
    /// Attachments recorded on the fixture.
    pub attachments: Vec<Attachment>,
    /// Fixture parameters.
    pub parameters: Vec<Parameter>,
    /// Start timestamp in milliseconds since the Unix epoch.
    pub start: Option<i64>,
    /// Stop timestamp in milliseconds since the Unix epoch.
    pub stop: Option<i64>,
}

/// Serialized Allure container result linking tests and fixtures.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResultContainer {
    /// Unique container identifier.
    pub uuid: String,
    /// Optional container display name.
    pub name: Option<String>,
    /// UUIDs of test results linked to this container.
    pub children: Vec<String>,
    /// Markdown container description.
    pub description: Option<String>,
    /// HTML container description.
    pub description_html: Option<String>,
    /// Before fixtures owned by this container.
    pub befores: Vec<FixtureResult>,
    /// After fixtures owned by this container.
    pub afters: Vec<FixtureResult>,
    /// Links attached at container scope.
    pub links: Vec<Link>,
    /// Start timestamp in milliseconds since the Unix epoch.
    pub start: Option<i64>,
    /// Stop timestamp in milliseconds since the Unix epoch.
    pub stop: Option<i64>,
}

/// Run-level diagnostics stored outside a single test result.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Globals {
    /// Run-level attachments.
    pub attachments: Vec<GlobalAttachment>,
    /// Run-level errors.
    pub errors: Vec<GlobalError>,
}

/// Run-level attachment entry.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalAttachment {
    /// Attachment display name.
    pub name: String,
    /// Attachment source filename in the results directory.
    pub source: String,
    /// Attachment content type.
    pub content_type: String,
}

/// Run-level error entry.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalError {
    /// Error message.
    pub message: String,
    /// Optional stack trace or diagnostic text.
    pub trace: Option<String>,
}

/// Allure categories file wrapper.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Categories(pub Vec<Category>);

/// Category rule used by Allure to group failures.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    /// Category display name.
    pub name: String,
    /// Optional category description.
    pub description: Option<String>,
    /// Statuses matched by this category.
    pub matched_statuses: Option<Vec<Status>>,
    /// Regular expression matched against status messages.
    pub message_regex: Option<String>,
    /// Regular expression matched against traces.
    pub trace_regex: Option<String>,
    /// Whether matched tests should be marked flaky.
    pub flaky: Option<bool>,
}

/// Allure execution status.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Successful execution.
    Passed,
    /// Assertion-like failure.
    Failed,
    /// Unexpected or infrastructure failure.
    Broken,
    /// Skipped, disabled, pending, or intentionally not executed.
    Skipped,
}

/// Allure lifecycle stage.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    /// Known before execution starts.
    Scheduled,
    /// Currently executing.
    Running,
    /// Fully completed.
    Finished,
    /// Not executed yet or pending.
    Pending,
    /// Aborted unexpectedly.
    Interrupted,
}

/// Additional details for a status.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusDetails {
    /// Human-readable status message.
    pub message: Option<String>,
    /// Stack trace or diagnostic trace.
    pub trace: Option<String>,
    /// Actual value captured for assertion-style failures.
    pub actual: Option<String>,
    /// Expected value captured for assertion-style failures.
    pub expected: Option<String>,
}

/// Allure label.
#[derive(Debug, Clone, Serialize)]
pub struct Label {
    /// Label name.
    pub name: String,
    /// Label value.
    pub value: String,
}

/// Allure link.
#[derive(Debug, Clone, Serialize)]
pub struct Link {
    /// Optional display name.
    pub name: Option<String>,
    /// Link URL.
    pub url: String,
    /// Optional link type such as `issue` or `tms`.
    #[serde(rename = "type")]
    pub link_type: Option<String>,
}

/// Allure parameter.
#[derive(Debug, Clone, Serialize)]
pub struct Parameter {
    /// Parameter name.
    pub name: String,
    /// Parameter value.
    pub value: String,
    /// Whether the parameter is excluded from history identity.
    pub excluded: Option<bool>,
    /// Optional display mode.
    pub mode: Option<ParameterMode>,
}

/// Parameter display mode.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterMode {
    /// Show the value normally.
    Default,
    /// Show the parameter name but hide the value.
    Masked,
    /// Hide the parameter from report display.
    Hidden,
}

/// Allure attachment reference.
#[derive(Debug, Clone, Serialize)]
pub struct Attachment {
    /// Attachment display name.
    pub name: String,
    /// Attachment source filename in the results directory.
    pub source: String,
    /// Attachment content type.
    #[serde(rename = "type")]
    pub content_type: String,
}

/// Serialized Allure step result.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepResult {
    /// Optional step identifier.
    pub uuid: Option<String>,
    /// Step display name.
    pub name: String,
    /// Final step status.
    pub status: Option<Status>,
    /// Details associated with the final step status.
    pub status_details: Option<StatusDetails>,
    /// Lifecycle stage of the step.
    pub stage: Option<Stage>,
    /// Nested steps.
    pub steps: Vec<StepResult>,
    /// Attachments stored on the step.
    pub attachments: Vec<Attachment>,
    /// Step-level parameters.
    pub parameters: Vec<Parameter>,
    /// Start timestamp in milliseconds since the Unix epoch.
    pub start: Option<i64>,
    /// Stop timestamp in milliseconds since the Unix epoch.
    pub stop: Option<i64>,
}
