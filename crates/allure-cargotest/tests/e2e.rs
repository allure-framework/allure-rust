use allure_rust_commons::md5_hex;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug)]
struct TempProjectDir {
    path: PathBuf,
}

impl TempProjectDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&path).expect("temp project dir should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempProjectDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn generates_allure_results_for_descriptions_sample() {
    let (results, _, _project_dir) = run_sample("descriptions", true);
    let result = results
        .get("writes_descriptions")
        .expect("missing writes_descriptions result");

    assert_has_allure_result_fields(result);
    assert_eq!(json_string(result, "status"), Some("passed"));
    assert_eq!(
        json_string(result, "description"),
        Some("markdown description")
    );
    assert_eq!(
        json_string(result, "descriptionHtml"),
        Some("<p>html description</p>")
    );
}

#[test]
fn generates_allure_results_for_labels_sample() {
    let (results, _, _project_dir) = run_sample("labels", true);
    let result = results
        .get("writes_all_labels")
        .expect("missing writes_all_labels result");

    assert_has_allure_result_fields(result);
    assert_eq!(json_string(result, "status"), Some("passed"));

    assert!(contains_label(result, "custom", "v1"));
    assert!(contains_label(result, "team", "qa"));
    assert!(contains_label(result, "component", "billing"));
    assert!(contains_label(result, "epic", "checkout"));
    assert!(contains_label(result, "feature", "payment"));
    assert!(contains_label(result, "story", "pay by card"));
    assert!(contains_label(result, "suite", "api-suite"));
    assert!(contains_label(result, "parentSuite", "integration"));
    assert!(contains_label(result, "subSuite", "card-flows"));
    assert!(contains_label(result, "owner", "alice"));
    assert!(contains_label(result, "severity", "critical"));
    assert!(contains_label(result, "layer", "e2e"));
    assert!(contains_label(result, "tag", "smoke"));
    assert!(contains_label(result, "tag", "regression"));
    assert!(contains_label(result, "ALLURE_ID", "T-42"));
    assert!(result.contains("\"url\":\"https://example.test/wiki\",\"type\":\"custom\""));
    assert!(result.contains("\"url\":\"https://example.test/issue/456\",\"type\":\"issue\""));
}

#[test]
fn generates_synthetic_suite_labels_when_not_overridden() {
    let (results, _, _project_dir) = run_sample("labels", true);
    let result = results
        .get("derives_synthetic_suite_labels_by_default")
        .expect("missing derives_synthetic_suite_labels_by_default result");

    assert_has_allure_result_fields(result);
    assert_eq!(json_string(result, "status"), Some("passed"));

    assert!(contains_label(result, "suite", "allure"));
    assert!(!contains_label_name(result, "parentSuite"));
    assert!(!contains_label_name(result, "subSuite"));
}

#[test]
fn generates_allure_results_for_links_sample() {
    let (results, _, _project_dir) = run_sample("links", true);
    let result = results
        .get("writes_links")
        .expect("missing writes_links result");

    assert_has_allure_result_fields(result);
    assert_eq!(json_string(result, "status"), Some("passed"));

    assert!(result.contains(
        "\"links\":[{\"name\":\"docs\",\"url\":\"https://example.test/docs\",\"type\":\"custom\"}"
    ));
    assert!(result.contains("\"url\":\"https://example.test/issue/123\",\"type\":\"issue\""));
    assert!(result.contains("\"url\":\"https://example.test/tms/456\",\"type\":\"tms\""));
}

#[test]
fn generates_allure_results_for_parameters_sample() {
    let (results, _, _project_dir) = run_sample("parameters", true);
    let result = results
        .get("writes_parameters")
        .expect("missing writes_parameters result");

    assert_has_allure_result_fields(result);
    assert_eq!(json_string(result, "status"), Some("passed"));
    assert!(result.contains(
        "\"parameters\":[{\"name\":\"browser\",\"value\":\"firefox\",\"excluded\":null,\"mode\":null}"
    ));
    assert!(
        result.contains("{\"name\":\"retries\",\"value\":\"2\",\"excluded\":null,\"mode\":null}]")
    );
}

#[test]
fn generates_allure_results_for_attachments_sample() {
    let (results, results_dir, _project_dir) = run_sample("attachments", true);
    let result = results
        .get("writes_attachment")
        .expect("missing writes_attachment result");

    assert_has_allure_result_fields(result);
    assert_eq!(json_string(result, "status"), Some("passed"));
    assert!(result.contains("\"attachments\":[{\"name\":\"hello.txt\""));
    assert!(result.contains("\"type\":\"text/plain\""));

    let attachment_source = json_string(result, "source").expect("attachment source should exist");
    let attachment_path = results_dir.join(attachment_source);
    let attachment_content = fs::read_to_string(attachment_path).expect("attachment should exist");
    assert_eq!(attachment_content, "hello from attachments sample");
}

#[test]
fn generates_allure_results_for_steps_sample() {
    let (results, results_dir, _project_dir) = run_sample("steps", true);
    let result = results
        .get("writes_steps")
        .expect("missing writes_steps result");

    assert_has_allure_result_fields(result);
    assert_eq!(json_string(result, "status"), Some("passed"));

    assert!(
        result.contains("\"steps\":[{\"uuid\":null,\"name\":\"simple step\",\"status\":\"passed\"")
    );
    assert!(result.contains("{\"uuid\":null,\"name\":\"logged step\",\"status\":\"passed\""));
    assert!(result
        .contains("{\"uuid\":null,\"name\":\"some_step_doing_something\",\"status\":\"passed\""));
    assert!(
        result.contains("{\"uuid\":null,\"name\":\"Readable step title\",\"status\":\"passed\"")
    );
    assert!(result.contains("{\"uuid\":null,\"name\":\"failed step\",\"status\":\"failed\""));
    assert!(result.contains("\"statusDetails\":{\"message\":\"step failed\""));
    assert!(result.contains("{\"uuid\":null,\"name\":\"broken parent\",\"status\":\"broken\""));
    assert!(result.contains("\"statusDetails\":{\"message\":\"step broken\""));
    assert!(result
        .contains("\"steps\":[{\"uuid\":null,\"name\":\"nested child\",\"status\":\"passed\""));
    assert!(result.contains("\"attachments\":[{\"name\":\"nested.txt\""));

    let attachment_source = result
        .split("\"name\":\"nested.txt\",\"source\":\"")
        .nth(1)
        .and_then(|tail| tail.split('"').next())
        .expect("nested step attachment source should exist");

    let attachment_content = fs::read_to_string(results_dir.join(attachment_source))
        .expect("nested attachment should exist");
    assert_eq!(attachment_content, "inside nested step");
}

#[test]
fn generates_allure_results_for_failing_tests() {
    let (results, _, _project_dir) = run_sample("failing", false);

    let passing = results
        .get("still_writes_passed")
        .expect("missing still_writes_passed result");
    assert_has_allure_result_fields(passing);
    assert_eq!(json_string(passing, "status"), Some("passed"));

    let failing = results
        .get("fails_with_message")
        .expect("missing fails_with_message result");
    assert_has_allure_result_fields(failing);
    assert_eq!(json_string(failing, "status"), Some("failed"));
    assert_eq!(
        json_string(failing, "message"),
        Some("expected failure from sample")
    );
}

#[test]
fn generates_allure_results_for_should_panic_tests() {
    let (results, _, _project_dir) = run_sample("should_panic", false);

    let should_panic_without_expected_passes = results
        .get("should_panic_without_expected_passes")
        .expect("missing should_panic_without_expected_passes result");
    assert_has_allure_result_fields(should_panic_without_expected_passes);
    assert_eq!(
        json_string(should_panic_without_expected_passes, "status"),
        Some("passed")
    );

    let should_panic_with_expected_passes = results
        .get("should_panic_with_expected_passes")
        .expect("missing should_panic_with_expected_passes result");
    assert_has_allure_result_fields(should_panic_with_expected_passes);
    assert_eq!(
        json_string(should_panic_with_expected_passes, "status"),
        Some("passed")
    );

    let should_panic_with_expected_mismatch_fails = results
        .get("should_panic_with_expected_mismatch_fails")
        .expect("missing should_panic_with_expected_mismatch_fails result");
    assert_has_allure_result_fields(should_panic_with_expected_mismatch_fails);
    assert_eq!(
        json_string(should_panic_with_expected_mismatch_fails, "status"),
        Some("failed")
    );
    assert!(should_panic_with_expected_mismatch_fails
        .contains("panic message mismatch: expected substring"));
    assert!(should_panic_with_expected_mismatch_fails.contains("needle"));
    assert!(should_panic_with_expected_mismatch_fails.contains("different panic message"));

    let should_panic_without_panic_fails = results
        .get("should_panic_without_panic_fails")
        .expect("missing should_panic_without_panic_fails result");
    assert_has_allure_result_fields(should_panic_without_panic_fails);
    assert_eq!(
        json_string(should_panic_without_panic_fails, "status"),
        Some("failed")
    );
    assert_eq!(
        json_string(should_panic_without_panic_fails, "message"),
        Some("expected panic but none occurred")
    );
}

#[test]
fn generates_default_runtime_labels() {
    let (results, _, _project_dir) = run_sample("default_and_global_labels", true);
    let result = results
        .get("writes_default_and_global_labels")
        .expect("missing writes_default_and_global_labels result");

    assert_has_allure_result_fields(result);
    assert_eq!(json_string(result, "status"), Some("passed"));

    assert!(contains_label(result, "language", "rust"));
    assert!(contains_label(result, "framework", "cargo-test"));
    assert!(contains_label_name(result, "host"));
    assert!(contains_label_name(result, "thread"));
}

#[test]
fn generates_global_labels_and_runtime_overrides() {
    let mut envs = HashMap::new();
    envs.insert("ALLURE_LABEL_component", "checkout");
    envs.insert("allure.label.layer", "e2e");
    envs.insert("ALLURE_HOST_NAME", "ci-host");
    envs.insert("ALLURE_THREAD_NAME", "worker-7");

    let (results, _, _project_dir) = run_sample_with_env("default_and_global_labels", true, &envs);
    let result = results
        .get("writes_default_and_global_labels")
        .expect("missing writes_default_and_global_labels result");

    assert_has_allure_result_fields(result);
    assert_eq!(json_string(result, "status"), Some("passed"));

    assert!(contains_label(result, "host", "ci-host"));
    assert!(contains_label(result, "thread", "worker-7"));
    assert!(contains_label(result, "component", "checkout"));
    assert!(contains_label(result, "layer", "e2e"));
}

#[test]
fn selective_run_when_test_selected_by_name() {
    let testplan = r#"{"version":"1.0","tests":[{"selector":"allure::selected_by_name"}]}"#;
    let (results, _, _project_dir) = run_sample_with_testplan("selective", Some(testplan), true);

    assert_eq!(results.len(), 1);
    assert!(results.contains_key("selected_by_name"));
}

#[test]
fn selective_run_does_not_match_partial_selector_name() {
    let testplan = r#"{"version":"1.0","tests":[{"selector":"selected_by_name"}]}"#;
    let (results, _, _project_dir) = run_sample_with_testplan("selective", Some(testplan), true);

    assert!(results.is_empty());
}

#[test]
fn generates_allure_id_label_from_macro_attribute() {
    let (results, _, _project_dir) = run_sample("selective", true);

    let result = results
        .get("selected_by_id")
        .expect("missing selected_by_id result");
    assert!(contains_label(result, "ALLURE_ID", "A-2"));
}

#[test]
fn selective_run_when_multiple_tests_selected() {
    let testplan =
        r#"{"version":"1.0","tests":[{"selector":"allure::selected_by_name"},{"id":"A-2"}]}"#;
    let (results, _, _project_dir) = run_sample_with_testplan("selective", Some(testplan), true);

    assert_eq!(results.len(), 1);
    assert!(results.contains_key("selected_by_name"));
}

#[test]
fn selective_run_when_no_tests_selected() {
    let testplan = r#"{"version":"1.0","tests":[{"selector":"missing::test"}]}"#;
    let (results, _, _project_dir) = run_sample_with_testplan("selective", Some(testplan), true);

    assert!(results.is_empty());
}

#[test]
fn selective_run_with_malformed_testplan() {
    let (results, _, _project_dir) = run_sample_with_testplan("selective", Some("not json"), true);

    assert_eq!(results.len(), 3);
    assert!(results.contains_key("selected_by_name"));
    assert!(results.contains_key("selected_by_id"));
    assert!(results.contains_key("selected_extra"));
}

#[test]
fn selective_run_with_missing_testplan_file_and_env_var() {
    let mut envs = HashMap::new();
    envs.insert(
        "ALLURE_TESTPLAN_PATH",
        "/tmp/allure-missing-testplan-does-not-exist.json",
    );

    let (results, _, _project_dir) = run_sample_with_env_allow_empty("selective", true, &envs);

    assert_eq!(results.len(), 3);
    assert!(results.contains_key("selected_by_name"));
    assert!(results.contains_key("selected_by_id"));
    assert!(results.contains_key("selected_extra"));
}

#[test]
fn keeps_metadata_linked_to_the_right_test_when_running_concurrently() {
    let mut envs = HashMap::new();
    envs.insert("RUST_TEST_THREADS", "2");

    let (results, _, _project_dir) = run_sample_with_env("concurrent_metadata", true, &envs);

    let first = results
        .get("metadata_for_first_test_stays_isolated")
        .expect("missing metadata_for_first_test_stays_isolated result");
    assert_has_allure_result_fields(first);
    assert!(contains_label(first, "component", "alpha"));
    assert!(first.contains(
        "\"parameters\":[{\"name\":\"case\",\"value\":\"alpha\",\"excluded\":null,\"mode\":null}]"
    ));
    assert!(first.contains("\"url\":\"https://example.test/alpha\""));
    assert!(!contains_label(first, "component", "beta"));
    assert!(!first.contains("\"value\":\"beta\""));
    assert!(!first.contains("https://example.test/beta"));

    let second = results
        .get("metadata_for_second_test_stays_isolated")
        .expect("missing metadata_for_second_test_stays_isolated result");
    assert_has_allure_result_fields(second);
    assert!(contains_label(second, "component", "beta"));
    assert!(second.contains(
        "\"parameters\":[{\"name\":\"case\",\"value\":\"beta\",\"excluded\":null,\"mode\":null}]"
    ));
    assert!(second.contains("\"url\":\"https://example.test/beta\""));
    assert!(!contains_label(second, "component", "alpha"));
    assert!(!second.contains("\"value\":\"alpha\""));
    assert!(!second.contains("https://example.test/alpha"));
}

fn run_sample_with_testplan(
    sample_name: &str,
    testplan_content: Option<&str>,
    expect_success: bool,
) -> (HashMap<String, String>, PathBuf, TempProjectDir) {
    let project_dir = prepare_sample_project(sample_name);
    let results_dir = project_dir.path().join("allure-results");

    let mut envs = HashMap::new();
    let mut plan_path = None;

    if let Some(content) = testplan_content {
        let path = project_dir.path().join("testplan.json");
        fs::write(&path, content).expect("test plan should be written");
        plan_path = Some(path);
    }

    if let Some(path) = &plan_path {
        envs.insert(
            "ALLURE_TESTPLAN_PATH",
            path.to_str().expect("path should be utf-8"),
        );
    }

    let output = run_cargo_test(project_dir.path(), &results_dir, &envs);
    if expect_success {
        assert!(
            output.status.success(),
            "cargo test failed for {sample_name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    } else {
        assert!(
            !output.status.success(),
            "cargo test unexpectedly succeeded for sample {sample_name}"
        );
    }

    (
        read_results_by_test_name_allow_empty(&results_dir),
        results_dir,
        project_dir,
    )
}

#[test]
fn generates_test_case_id_and_runtime_override() {
    let (results, _, _project_dir) = run_sample("identifiers", true);

    let generated = results
        .get("computes_test_case_id_from_full_name")
        .expect("missing computes_test_case_id_from_full_name result");
    assert_has_allure_result_fields(generated);
    assert_eq!(json_string(generated, "status"), Some("passed"));

    let full_name = json_string(generated, "fullName").expect("fullName should exist");
    let expected_test_case_id = md5_hex(full_name);
    assert_eq!(
        json_string(generated, "testCaseId"),
        Some(expected_test_case_id.as_str())
    );

    let overridden = results
        .get("allows_runtime_override_for_test_case_id")
        .expect("missing allows_runtime_override_for_test_case_id result");
    assert_has_allure_result_fields(overridden);
    assert_eq!(json_string(overridden, "status"), Some("passed"));
    assert_eq!(
        json_string(overridden, "testCaseId"),
        Some("runtime-overridden-test-case-id")
    );
}

fn run_sample(
    sample_name: &str,
    expect_success: bool,
) -> (HashMap<String, String>, PathBuf, TempProjectDir) {
    run_sample_with_env(sample_name, expect_success, &HashMap::new())
}

fn run_sample_with_env_allow_empty(
    sample_name: &str,
    expect_success: bool,
    envs: &HashMap<&str, &str>,
) -> (HashMap<String, String>, PathBuf, TempProjectDir) {
    let project_dir = prepare_sample_project(sample_name);
    let results_dir = project_dir.path().join("allure-results");

    let output = run_cargo_test(project_dir.path(), &results_dir, envs);
    if expect_success {
        assert!(
            output.status.success(),
            "cargo test failed for {sample_name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    } else {
        assert!(
            !output.status.success(),
            "cargo test unexpectedly succeeded for sample {sample_name}"
        );
    }

    (
        read_results_by_test_name_allow_empty(&results_dir),
        results_dir,
        project_dir,
    )
}

fn run_sample_with_env(
    sample_name: &str,
    expect_success: bool,
    envs: &HashMap<&str, &str>,
) -> (HashMap<String, String>, PathBuf, TempProjectDir) {
    let (results, results_dir, project_dir) =
        run_sample_with_env_allow_empty(sample_name, expect_success, envs);

    assert!(
        !results.is_empty(),
        "no result files found in {}",
        results_dir.display()
    );

    (results, results_dir, project_dir)
}

fn prepare_sample_project(sample_name: &str) -> TempProjectDir {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate should be nested under workspace")
        .parent()
        .expect("workspace root should exist")
        .to_path_buf();
    let samples_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("samples");

    let project_dir = TempProjectDir::new("allure-cargotest-e2e");
    let src_dir = project_dir.path().join("src");
    let tests_dir = project_dir.path().join("tests");

    fs::create_dir_all(&src_dir).expect("sample src dir should be created");
    fs::create_dir_all(&tests_dir).expect("sample tests dir should be created");

    fs::copy(
        samples_root.join("src").join("lib.rs"),
        src_dir.join("lib.rs"),
    )
    .expect("shared sample lib should be copied");
    fs::copy(
        samples_root.join("tests").join(format!("{sample_name}.rs")),
        tests_dir.join("allure.rs"),
    )
    .expect("sample test should be copied");

    let cargo_toml = format!(
        r#"[package]
name = "allure-cargotest-sample-{}"
version = "0.1.0"
edition = "2021"

[workspace]

[dependencies]
allure-cargotest = {{ path = "{}" }}
"#,
        sample_name,
        repo_root
            .join("crates")
            .join("allure-cargotest")
            .to_str()
            .expect("path should be utf-8")
    );
    fs::write(project_dir.path().join("Cargo.toml"), cargo_toml)
        .expect("sample Cargo.toml should be generated");

    project_dir
}

fn run_cargo_test(
    project_dir: &Path,
    results_dir: &Path,
    envs: &HashMap<&str, &str>,
) -> std::process::Output {
    let mut command = Command::new("cargo");
    command
        .arg("test")
        .arg("--")
        .arg("--nocapture")
        .env("ALLURE_RESULTS_DIR", results_dir)
        .current_dir(project_dir);

    for (key, value) in envs {
        command.env(key, value);
    }

    command.output().expect("cargo test should run")
}

fn read_results_by_test_name_allow_empty(results_dir: &Path) -> HashMap<String, String> {
    let mut parsed = HashMap::new();
    let mut result_files = Vec::new();

    for entry in fs::read_dir(results_dir).expect("results dir should exist") {
        let entry = entry.expect("entry should be readable");
        let path = entry.path();
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with("-result.json"))
        {
            result_files.push(path);
        }
    }

    for result_file in result_files {
        let raw = fs::read_to_string(&result_file).expect("result file should be readable");
        let name = json_string(&raw, "name")
            .expect("result name should be present")
            .to_string();
        parsed.insert(name, raw);
    }

    parsed
}

fn json_string<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let pattern = format!("\"{key}\":\"");
    let start = json.find(&pattern)? + pattern.len();
    let rest = &json[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

fn contains_label(json: &str, name: &str, value: &str) -> bool {
    json.contains(&format!("\"name\":\"{name}\",\"value\":\"{value}\""))
}

fn contains_label_name(json: &str, name: &str) -> bool {
    json.contains(&format!("\"name\":\"{name}\",\"value\":"))
}

fn assert_has_allure_result_fields(result: &str) {
    let expected = [
        "uuid",
        "name",
        "fullName",
        "historyId",
        "testCaseId",
        "description",
        "descriptionHtml",
        "status",
        "statusDetails",
        "labels",
        "links",
        "parameters",
        "steps",
        "attachments",
        "start",
        "stop",
    ];

    for field in expected {
        assert!(
            result.contains(&format!("\"{field}\":")),
            "expected field '{field}' to exist in result: {result}"
        );
    }
}
