use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub uuid: String,
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

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FixtureResult {
    pub name: String,
    pub status: Option<Status>,
    pub status_details: Option<StatusDetails>,
    pub stage: Option<Stage>,
    pub steps: Vec<StepResult>,
    pub attachments: Vec<Attachment>,
    pub parameters: Vec<Parameter>,
    pub start: Option<i64>,
    pub stop: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResultContainer {
    pub uuid: String,
    pub name: Option<String>,
    pub children: Vec<String>,
    pub description: Option<String>,
    pub description_html: Option<String>,
    pub befores: Vec<FixtureResult>,
    pub afters: Vec<FixtureResult>,
    pub links: Vec<Link>,
    pub start: Option<i64>,
    pub stop: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Globals {
    pub attachments: Vec<GlobalAttachment>,
    pub errors: Vec<GlobalError>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalAttachment {
    pub name: String,
    pub source: String,
    pub content_type: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalError {
    pub message: String,
    pub trace: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Categories(pub Vec<Category>);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub name: String,
    pub description: Option<String>,
    pub matched_statuses: Option<Vec<Status>>,
    pub message_regex: Option<String>,
    pub trace_regex: Option<String>,
    pub flaky: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Passed,
    Failed,
    Broken,
    Skipped,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Scheduled,
    Running,
    Finished,
    Pending,
    Interrupted,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusDetails {
    pub message: Option<String>,
    pub trace: Option<String>,
    pub actual: Option<String>,
    pub expected: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Label {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Link {
    pub name: Option<String>,
    pub url: String,
    #[serde(rename = "type")]
    pub link_type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Parameter {
    pub name: String,
    pub value: String,
    pub excluded: Option<bool>,
    pub mode: Option<ParameterMode>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterMode {
    Default,
    Masked,
    Hidden,
}

#[derive(Debug, Clone, Serialize)]
pub struct Attachment {
    pub name: String,
    pub source: String,
    #[serde(rename = "type")]
    pub content_type: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepResult {
    pub uuid: Option<String>,
    pub name: String,
    pub status: Option<Status>,
    pub status_details: Option<StatusDetails>,
    pub stage: Option<Stage>,
    pub steps: Vec<StepResult>,
    pub attachments: Vec<Attachment>,
    pub parameters: Vec<Parameter>,
    pub start: Option<i64>,
    pub stop: Option<i64>,
}
