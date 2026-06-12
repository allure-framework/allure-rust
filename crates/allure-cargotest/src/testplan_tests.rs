use std::{
    env, fs,
    path::PathBuf,
    sync::{Mutex, MutexGuard},
    time::{SystemTime, UNIX_EPOCH},
};

use allure_rust_commons as allure;

use super::{parse_test_plan, TestPlan};

static TESTPLAN_ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock_testplan_env() -> MutexGuard<'static, ()> {
    TESTPLAN_ENV_LOCK
        .lock()
        .expect("testplan env lock should not be poisoned")
}

fn temp_file_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("valid time")
        .as_nanos();
    env::temp_dir().join(format!("allure-testplan-{nanos}.json"))
}

fn with_env_var_disabled() {
    // SAFETY: tests in this crate run in a controlled process for this use.
    unsafe { env::remove_var("ALLURE_TESTPLAN_PATH") };
}

fn with_env_var(path: &std::path::Path) {
    // SAFETY: tests in this crate run in a controlled process for this use.
    unsafe { env::set_var("ALLURE_TESTPLAN_PATH", path) };
}

fn attach_text(name: impl Into<String>, body: impl AsRef<str>) {
    allure::attachment(name, "text/plain", body.as_ref().as_bytes());
}

fn attach_testplan_input(case_name: &str, input: &str) {
    attach_text(format!("{case_name} test-plan input"), input);
}

fn attach_testplan_result(case_name: &str, result: &Option<TestPlan>) {
    attach_text(
        format!("{case_name} parsed test-plan result"),
        format!("{result:#?}"),
    );
}

fn parse_test_plan_input(case_name: &str, input: &str) -> Option<TestPlan> {
    attach_testplan_input(case_name, input);
    let result = parse_test_plan(input);
    attach_testplan_result(case_name, &result);
    result
}

fn load_test_plan_from_env(case_name: &str) -> Option<TestPlan> {
    let env_value = env::var("ALLURE_TESTPLAN_PATH").unwrap_or_else(|_| "<unset>".to_string());
    attach_text(
        format!("{case_name} ALLURE_TESTPLAN_PATH"),
        format!("ALLURE_TESTPLAN_PATH={env_value}"),
    );
    let result = TestPlan::from_env();
    attach_testplan_result(case_name, &result);
    result
}

#[test]
#[crate::log_asserts]
fn returns_none_when_env_is_unset() {
    allure::test(|| {
        allure::description(
            "Verifies the adapter treats a missing ALLURE_TESTPLAN_PATH as no active test plan.",
        );
        let _guard = lock_testplan_env();
        with_env_var_disabled();
        assert!(load_test_plan_from_env("missing env var").is_none());
    });
}

#[test]
#[crate::log_asserts]
fn returns_none_when_file_does_not_exist() {
    allure::test(|| {
        allure::description(
            "Verifies a configured but missing test-plan file disables selection instead of failing the run.",
        );
        let _guard = lock_testplan_env();
        with_env_var(std::path::Path::new(
            "/tmp/this-file-should-not-exist-testplan.json",
        ));
        assert!(load_test_plan_from_env("missing test-plan file").is_none());
    });
}

#[test]
#[crate::log_asserts]
fn parses_plan_when_file_exists() {
    allure::test(|| {
        allure::description(
            "Verifies a valid test-plan file is loaded from the environment and preserves entries.",
        );
        let _guard = lock_testplan_env();
        let path = temp_file_path();
        let input = r#"{"version":"1.0","tests":[{"id":"42"},{"selector":"suite::test_name"}]}"#;
        attach_testplan_input("valid env file", input);
        fs::write(&path, input).expect("write plan");

        with_env_var(&path);
        let plan = load_test_plan_from_env("valid env file").expect("plan parsed");

        assert_eq!(plan.version.as_deref(), Some("1.0"));
        assert_eq!(plan.tests.len(), 2);

        let _ = fs::remove_file(path);
    });
}

#[test]
#[crate::log_asserts]
fn returns_none_for_malformed_json() {
    allure::test(|| {
        allure::description(
            "Verifies malformed test-plan JSON is ignored as unavailable selection data.",
        );
        let _guard = lock_testplan_env();
        let path = temp_file_path();
        let input = "not json";
        attach_testplan_input("malformed env file", input);
        fs::write(&path, input).expect("write invalid plan");

        with_env_var(&path);
        assert!(load_test_plan_from_env("malformed env file").is_none());

        let _ = fs::remove_file(path);
    });
}

#[test]
#[crate::log_asserts]
fn treats_empty_tests_as_unavailable() {
    allure::test(|| {
        allure::description("Verifies test plans without entries are treated as unavailable.");
        let plan = parse_test_plan_input("empty test list", r#"{"version":"1","tests":[]}"#);
        assert!(plan.is_none());
    });
}

#[test]
#[crate::log_asserts]
fn matches_by_exact_full_name_only() {
    allure::test(|| {
        allure::description("Verifies selector matching requires the exact full test name.");
        let plan = parse_test_plan_input(
            "exact selector",
            r#"{"version":"1","tests":[{"selector":"crate::module::test_case"}]}"#,
        )
        .expect("valid plan");

        assert!(plan.is_selected(Some("crate::module::test_case"), None, None));
        assert!(!plan.is_selected(Some("module::test_case"), None, None));
        assert!(!plan.is_selected(None, None, None));
    });
}

#[test]
#[crate::log_asserts]
fn prefers_id_match_over_selector_within_entry() {
    allure::test(|| {
        allure::description(
            "Verifies an explicit Allure id match takes precedence over the selector in the same entry.",
        );
        let plan = parse_test_plan_input(
            "id and selector entry",
            r#"{"version":"1","tests":[{"id":"777","selector":"crate::module::test_case"}]}"#,
        )
        .expect("valid plan");

        assert!(plan.is_selected(Some("different::name"), Some("777"), None));
        assert!(!plan.is_selected(Some("crate::module::test_case"), Some("999"), None));
    });
}

#[test]
#[crate::log_asserts]
fn falls_back_to_metadata_tags_for_allure_id() {
    allure::test(|| {
        allure::description(
            "Verifies metadata tags can provide the Allure id used for test-plan selection.",
        );
        let plan = parse_test_plan_input(
            "metadata tag fallback",
            r#"{"version":"1","tests":[{"id":"A-2"}]}"#,
        )
        .expect("valid plan");

        let tags = ["smoke", "@allure.id=A-2"];
        assert!(plan.is_selected(Some("crate::module::test_case"), None, Some(&tags)));

        let colon_tags = ["@allure.id:A-2"];
        assert!(plan.is_selected(None, None, Some(&colon_tags)));
    });
}

#[test]
#[crate::log_asserts]
fn explicit_adapter_id_takes_precedence_over_tag_fallback() {
    allure::test(|| {
        allure::description(
            "Verifies an adapter-provided Allure id overrides tag-derived fallback ids.",
        );
        let plan = parse_test_plan_input(
            "explicit adapter id",
            r#"{"version":"1","tests":[{"id":"A-2"}]}"#,
        )
        .expect("valid plan");

        let tags = ["@allure.id=A-2"];
        assert!(!plan.is_selected(None, Some("B-1"), Some(&tags)));
    });
}
