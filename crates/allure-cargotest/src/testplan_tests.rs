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

#[test]
fn returns_none_when_env_is_unset() {
    allure::test(|| {
        allure::stage("execute test body");
        let _guard = lock_testplan_env();
        with_env_var_disabled();
        assert!(TestPlan::from_env().is_none());
    });
}

#[test]
fn returns_none_when_file_does_not_exist() {
    allure::test(|| {
        allure::stage("execute test body");
        let _guard = lock_testplan_env();
        with_env_var(std::path::Path::new(
            "/tmp/this-file-should-not-exist-testplan.json",
        ));
        assert!(TestPlan::from_env().is_none());
    });
}

#[test]
fn parses_plan_when_file_exists() {
    allure::test(|| {
        allure::stage("execute test body");
        let _guard = lock_testplan_env();
        let path = temp_file_path();
        fs::write(
            &path,
            r#"{"version":"1.0","tests":[{"id":"42"},{"selector":"suite::test_name"}]}"#,
        )
        .expect("write plan");

        with_env_var(&path);
        let plan = TestPlan::from_env().expect("plan parsed");

        assert_eq!(plan.version.as_deref(), Some("1.0"));
        assert_eq!(plan.tests.len(), 2);

        let _ = fs::remove_file(path);
    });
}

#[test]
fn returns_none_for_malformed_json() {
    allure::test(|| {
        allure::stage("execute test body");
        let _guard = lock_testplan_env();
        let path = temp_file_path();
        fs::write(&path, "not json").expect("write invalid plan");

        with_env_var(&path);
        assert!(TestPlan::from_env().is_none());

        let _ = fs::remove_file(path);
    });
}

#[test]
fn treats_empty_tests_as_unavailable() {
    allure::test(|| {
        allure::stage("execute test body");
        assert!(parse_test_plan(r#"{"version":"1","tests":[]}"#).is_none());
    });
}

#[test]
fn matches_by_exact_full_name_only() {
    allure::test(|| {
        allure::stage("execute test body");
        let plan =
            parse_test_plan(r#"{"version":"1","tests":[{"selector":"crate::module::test_case"}]}"#)
                .expect("valid plan");

        assert!(plan.is_selected(Some("crate::module::test_case"), None, None));
        assert!(!plan.is_selected(Some("module::test_case"), None, None));
        assert!(!plan.is_selected(None, None, None));
    });
}

#[test]
fn prefers_id_match_over_selector_within_entry() {
    allure::test(|| {
        allure::stage("execute test body");
        let plan = parse_test_plan(
            r#"{"version":"1","tests":[{"id":"777","selector":"crate::module::test_case"}]}"#,
        )
        .expect("valid plan");

        assert!(plan.is_selected(Some("different::name"), Some("777"), None));
        assert!(!plan.is_selected(Some("crate::module::test_case"), Some("999"), None));
    });
}

#[test]
fn falls_back_to_metadata_tags_for_allure_id() {
    allure::test(|| {
        allure::stage("execute test body");
        let plan =
            parse_test_plan(r#"{"version":"1","tests":[{"id":"A-2"}]}"#).expect("valid plan");

        let tags = ["smoke", "@allure.id=A-2"];
        assert!(plan.is_selected(Some("crate::module::test_case"), None, Some(&tags)));

        let colon_tags = ["@allure.id:A-2"];
        assert!(plan.is_selected(None, None, Some(&colon_tags)));
    });
}

#[test]
fn explicit_adapter_id_takes_precedence_over_tag_fallback() {
    allure::test(|| {
        allure::stage("execute test body");
        let plan =
            parse_test_plan(r#"{"version":"1","tests":[{"id":"A-2"}]}"#).expect("valid plan");

        let tags = ["@allure.id=A-2"];
        assert!(!plan.is_selected(None, Some("B-1"), Some(&tags)));
    });
}
