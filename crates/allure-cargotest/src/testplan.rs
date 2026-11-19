use std::{env, fs, path::PathBuf, sync::OnceLock};

use allure_rust_commons::{AllureFacade, GlobalError};

const TESTPLAN_ENV_VAR: &str = "ALLURE_TESTPLAN_PATH";

/// Allure test plan parsed from `ALLURE_TESTPLAN_PATH`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestPlan {
    /// Optional test plan version.
    pub version: Option<String>,
    /// Entries included in the plan.
    pub tests: Vec<TestPlanEntry>,
    /// Entries whose selected test bodies must not execute.
    pub muted_tests: Vec<TestPlanEntry>,
}

/// Minimal test entry shape used for matching test execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestPlanEntry {
    /// Optional Allure ID.
    pub id: Option<String>,
    /// Optional framework selector.
    pub selector: Option<String>,
}

impl TestPlan {
    /// Loads a test plan from `ALLURE_TESTPLAN_PATH`.
    ///
    /// Returns `None` when the env var is unset, when the path does not exist,
    /// or when the file cannot be parsed as a valid test plan JSON.
    ///
    /// Malformed JSON is treated as a non-fatal warning and does not panic test execution.
    pub fn from_env() -> Option<Self> {
        match load_test_plan_from_env() {
            Ok(plan) => plan,
            Err(error) => {
                eprintln!("warning: {error}");
                None
            }
        }
    }

    /// Returns `true` when the test should be executed according to the plan.
    ///
    /// Matching prefers explicit adapter ids, then optional metadata-derived ids,
    /// and finally exact `full_name` identity via `selector`.
    pub fn is_selected(
        &self,
        full_name: Option<&str>,
        allure_id: Option<&str>,
        tags: Option<&[&str]>,
    ) -> bool {
        let effective_id = effective_allure_id(allure_id, tags);

        self.tests.is_empty()
            || self.tests.iter().any(|entry| {
                if let Some(entry_id) = entry.id.as_deref() {
                    return effective_id.is_some_and(|candidate| candidate == entry_id);
                }

                entry
                    .selector
                    .as_deref()
                    .zip(full_name)
                    .is_some_and(|(selector, identity)| selector == identity)
            })
    }
    /// Returns whether either the effective ID or exact selector matches a muted entry.
    ///
    /// Explicit IDs override tag-derived IDs. Muting does not affect selection.
    pub fn is_muted(
        &self,
        full_name: Option<&str>,
        allure_id: Option<&str>,
        tags: Option<&[&str]>,
    ) -> bool {
        let effective_id = effective_allure_id(allure_id, tags);
        self.muted_tests.iter().any(|entry| {
            entry
                .id
                .as_deref()
                .zip(effective_id)
                .is_some_and(|(id, candidate)| id == candidate)
                || entry
                    .selector
                    .as_deref()
                    .zip(full_name)
                    .is_some_and(|(selector, identity)| selector == identity)
        })
    }
}

#[derive(Debug)]
struct TestPlanLoadError {
    message: String,
}

impl std::fmt::Display for TestPlanLoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

fn load_test_plan_from_env() -> Result<Option<TestPlan>, TestPlanLoadError> {
    let Some(path) = env::var_os(TESTPLAN_ENV_VAR).map(PathBuf::from) else {
        return Ok(None);
    };
    let context = "Allure test plan initialization failed; test selection was not applied";
    let body = fs::read_to_string(&path).map_err(|error| TestPlanLoadError {
        message: format!("{context}: could not read {}: {error}", path.display()),
    })?;
    let plan = parse_test_plan(&body).ok_or_else(|| TestPlanLoadError {
        message: format!(
            "{context}: could not parse test plan JSON from {}",
            path.display()
        ),
    })?;
    Ok(Some(plan))
}

pub(crate) fn load_test_plan_for_reporter(allure: &AllureFacade) -> Option<TestPlan> {
    match load_test_plan_from_env() {
        Ok(plan) => plan,
        Err(error) => {
            report_test_plan_error(error, |global_error| {
                allure.report_global_error(global_error)
            });
            None
        }
    }
}

pub(crate) fn active_test_plan() -> Option<&'static TestPlan> {
    static ACTIVE_TEST_PLAN: OnceLock<Option<TestPlan>> = OnceLock::new();

    ACTIVE_TEST_PLAN
        .get_or_init(|| match load_test_plan_from_env() {
            Ok(plan) => plan,
            Err(error) => {
                report_test_plan_error(error, allure_rust_commons::report_global_error);
                None
            }
        })
        .as_ref()
}

fn report_test_plan_error(
    error: TestPlanLoadError,
    report: impl FnOnce(GlobalError) -> std::io::Result<()>,
) {
    eprintln!("warning: {error}");
    if let Err(report_error) = report(GlobalError::new(error.to_string())) {
        eprintln!(
            "warning: failed to report Allure test plan initialization error: {report_error}"
        );
    }
}

fn parse_test_plan(input: &str) -> Option<TestPlan> {
    let value: serde_json::Value = serde_json::from_str(input).ok()?;
    let object = value.as_object()?;
    let version = object
        .get("version")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let tests = parse_entries(object.get("tests"))?;
    let muted_tests = parse_entries(object.get("excludedTests"))?;
    if tests.is_empty() && muted_tests.is_empty() {
        return None;
    }
    Some(TestPlan {
        version,
        tests,
        muted_tests,
    })
}

fn parse_entries(value: Option<&serde_json::Value>) -> Option<Vec<TestPlanEntry>> {
    let Some(value) = value else {
        return Some(Vec::new());
    };
    value
        .as_array()?
        .iter()
        .map(|value| {
            let entry = value.as_object()?;
            let id = match entry.get("id") {
                None | Some(serde_json::Value::Null) => None,
                Some(serde_json::Value::String(id)) => Some(id.clone()),
                Some(serde_json::Value::Number(id)) => Some(id.to_string()),
                _ => return None,
            };
            let selector = match entry.get("selector") {
                None | Some(serde_json::Value::Null) => None,
                Some(serde_json::Value::String(selector)) => Some(selector.clone()),
                _ => return None,
            };
            Some(TestPlanEntry { id, selector })
        })
        .collect()
}

fn effective_allure_id<'a>(
    allure_id: Option<&'a str>,
    tags: Option<&'a [&'a str]>,
) -> Option<&'a str> {
    allure_id.or_else(|| tags.and_then(allure_id_from_tags))
}

fn allure_id_from_tags<'a>(tags: &'a [&'a str]) -> Option<&'a str> {
    tags.iter().find_map(|tag| {
        tag.strip_prefix("@allure.id=")
            .or_else(|| tag.strip_prefix("@allure.id:"))
            .filter(|value| !value.is_empty())
    })
}

#[cfg(test)]
#[path = "testplan_tests.rs"]
mod testplan_tests;
