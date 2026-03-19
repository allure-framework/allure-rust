use std::{env, fs, path::PathBuf};

const TESTPLAN_ENV_VAR: &str = "ALLURE_TESTPLAN_PATH";

/// Allure test plan parsed from `ALLURE_TESTPLAN_PATH`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestPlan {
    pub version: Option<String>,
    pub tests: Vec<TestPlanEntry>,
}

/// Minimal test entry shape used for matching test execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestPlanEntry {
    pub id: Option<String>,
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
        let path = env::var_os(TESTPLAN_ENV_VAR).map(PathBuf::from)?;
        if !path.exists() {
            return None;
        }

        let body = match fs::read_to_string(&path) {
            Ok(body) => body,
            Err(err) => {
                eprintln!(
                    "warning: failed to read {} from {}: {err}",
                    TESTPLAN_ENV_VAR,
                    path.display()
                );
                return None;
            }
        };

        match parse_test_plan(&body) {
            Some(plan) => Some(plan),
            None => {
                eprintln!(
                    "warning: failed to parse test plan JSON from {} ({})",
                    TESTPLAN_ENV_VAR,
                    path.display()
                );
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

        self.tests.iter().any(|entry| {
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
}

fn parse_test_plan(input: &str) -> Option<TestPlan> {
    let compact: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    if !compact.starts_with('{') || !compact.ends_with('}') {
        return None;
    }

    if !compact.contains("\"tests\":") {
        return None;
    }

    let version = extract_string_field(&compact, "version");
    let tests_blob = extract_array_field(&compact, "tests")?;
    let tests = parse_tests_array(tests_blob)?;
    if tests.is_empty() {
        return None;
    }

    Some(TestPlan { version, tests })
}

fn parse_tests_array(tests_blob: &str) -> Option<Vec<TestPlanEntry>> {
    let mut tests = Vec::new();
    let mut depth = 0usize;
    let mut start = None;

    for (idx, ch) in tests_blob.char_indices() {
        match ch {
            '{' => {
                if depth == 0 {
                    start = Some(idx);
                }
                depth += 1;
            }
            '}' => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
                if depth == 0 {
                    let object_start = start?;
                    let object = &tests_blob[object_start..=idx];
                    tests.push(TestPlanEntry {
                        id: extract_string_field(object, "id"),
                        selector: extract_string_field(object, "selector"),
                    });
                    start = None;
                }
            }
            _ => {}
        }
    }

    if depth != 0 {
        return None;
    }

    Some(tests)
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

fn extract_array_field<'a>(json: &'a str, field_name: &str) -> Option<&'a str> {
    let key = format!("\"{field_name}\":[");
    let start = json.find(&key)? + key.len();
    let mut depth = 1usize;

    for (offset, ch) in json[start..].char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&json[start..start + offset]);
                }
            }
            _ => {}
        }
    }

    None
}

fn extract_string_field(json: &str, field_name: &str) -> Option<String> {
    let key = format!("\"{field_name}\":\"");
    let start = json.find(&key)? + key.len();
    let rest = &json[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

#[cfg(test)]
#[path = "testplan_tests.rs"]
mod testplan_tests;
