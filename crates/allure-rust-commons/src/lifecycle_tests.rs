use super::*;
use std::{fs, path::PathBuf};

fn reset_active_roots() {
    ACTIVE_TEST_ROOT.with(|cell| *cell.borrow_mut() = None);
    ACTIVE_SCOPE_ROOT.with(|cell| *cell.borrow_mut() = None);
}

fn make_lifecycle(test_name: &str) -> (AllureLifecycle, PathBuf) {
    reset_active_roots();
    let out_dir = std::env::temp_dir().join(format!(
        "allure-rust-lifecycle-tests-{test_name}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos()
    ));
    let runtime = AllureRuntime::new(
        FileSystemResultsWriter::new(&out_dir).expect("writer should initialize"),
    );
    (runtime.lifecycle(), out_dir)
}

fn read_jsons_with_suffix(out_dir: &PathBuf, suffix: &str) -> Vec<serde_json::Value> {
    let mut values = fs::read_dir(out_dir)
        .expect("output dir should exist")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.ends_with(suffix))
                .unwrap_or(false)
        })
        .map(|path| {
            let text = fs::read_to_string(path).expect("json file should be readable");
            serde_json::from_str::<serde_json::Value>(&text).expect("json file should be valid")
        })
        .collect::<Vec<_>>();
    values.sort_by_key(|v| v["uuid"].as_str().unwrap_or_default().to_string());
    values
}

#[test]
fn test_case_public_methods_are_persisted() {
    let (lifecycle, out_dir) = make_lifecycle("test-case-public-methods");

    lifecycle.start_test_case("api-test");
    let test_uuid = lifecycle
        .current_test_uuid()
        .expect("current_test_uuid should be available after start");
    lifecycle.update_test_case(|test| test.description = Some("description".to_string()));
    lifecycle.set_test_case_id("explicit-case-id");
    lifecycle.add_label("suite", "commons");
    lifecycle.add_link(
        "https://example.invalid/case",
        Some("case-link".to_string()),
        Some("tms".to_string()),
    );
    lifecycle.add_parameter("browser", "firefox");
    lifecycle.start_step("root step");
    lifecycle.add_attachment("trace.txt", "text/plain", b"trace body");
    lifecycle.stop_step(Status::Passed, None);
    lifecycle.stop_test_case(Status::Passed, None);

    assert!(lifecycle.current_test_uuid().is_none());

    let results = read_jsons_with_suffix(&out_dir, "-result.json");
    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert_eq!(result["uuid"], test_uuid);
    assert_eq!(result["name"], "api-test");
    assert_eq!(result["description"], "description");
    assert_eq!(result["testCaseId"], "explicit-case-id");
    assert_eq!(result["labels"][0]["name"], "suite");
    assert_eq!(result["links"][0]["url"], "https://example.invalid/case");
    assert_eq!(result["parameters"][0]["name"], "browser");
    assert_eq!(result["steps"][0]["name"], "root step");
    assert_eq!(result["steps"][0]["status"], "passed");
}

#[test]
fn start_test_case_accepts_optional_test_result_fields() {
    let (lifecycle, out_dir) = make_lifecycle("start-test-case-params");

    lifecycle.start_test_case(StartTestCaseParams {
        uuid: Some("custom-uuid".to_string()),
        name: "display name".to_string(),
        full_name: Some("pkg::display_name".to_string()),
        history_id: Some("custom-history".to_string()),
        test_case_id: Some("custom-case".to_string()),
        description: Some("markdown".to_string()),
        description_html: Some("<p>html</p>".to_string()),
        status: Some(Status::Skipped),
        status_details: Some(StatusDetails {
            message: Some("preset".to_string()),
            trace: None,
            actual: None,
            expected: None,
        }),
        stage: Some(Stage::Pending),
        labels: vec![Label {
            name: "suite".to_string(),
            value: "commons".to_string(),
        }],
        links: vec![Link {
            name: Some("docs".to_string()),
            url: "https://example.invalid/docs".to_string(),
            link_type: Some("custom".to_string()),
        }],
        parameters: vec![Parameter {
            name: "browser".to_string(),
            value: "firefox".to_string(),
            excluded: None,
            mode: None,
        }],
        steps: vec![StepResult {
            name: "seed step".to_string(),
            ..Default::default()
        }],
        attachments: vec![Attachment {
            name: "seed.txt".to_string(),
            source: "seed-source.txt".to_string(),
            content_type: "text/plain".to_string(),
        }],
        title_path: Some(vec!["module".to_string(), "test".to_string()]),
        start: Some(100),
        stop: Some(200),
    });
    lifecycle.stop_test_case(Status::Passed, None);

    let results = read_jsons_with_suffix(&out_dir, "-result.json");
    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert_eq!(result["uuid"], "custom-uuid");
    assert_eq!(result["fullName"], "pkg::display_name");
    let parameter_hash = md5_hex("browser:firefox");
    let expected_history_id = md5_hex(&format!("custom-case:{parameter_hash}"));
    assert_eq!(result["historyId"], expected_history_id);
    assert_eq!(result["testCaseId"], "custom-case");
    assert_eq!(result["description"], "markdown");
    assert_eq!(result["descriptionHtml"], "<p>html</p>");
    assert_eq!(result["status"], "passed");
    assert_eq!(result["labels"][0]["value"], "commons");
    assert_eq!(result["links"][0]["name"], "docs");
    assert_eq!(result["parameters"][0]["name"], "browser");
    assert_eq!(result["steps"][0]["name"], "seed step");
    assert_eq!(result["attachments"][0]["name"], "seed.txt");
    assert_eq!(result["titlePath"][0], "module");
    assert_eq!(result["start"], 100);
    assert_eq!(result["stop"], 200);
}

#[test]
fn start_with_full_name_derives_test_case_id_and_finalizes_dangling_steps() {
    let (lifecycle, out_dir) = make_lifecycle("full-name-and-finalize");

    lifecycle.start_test_case(StartTestCaseParams::new("display").with_full_name("pkg::display"));
    lifecycle.start_step("dangling");
    lifecycle.stop_test_case(Status::Failed, None);

    let results = read_jsons_with_suffix(&out_dir, "-result.json");
    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert_eq!(result["fullName"], "pkg::display");
    assert_eq!(result["testCaseId"], md5_hex("pkg::display"));
    assert_eq!(result["steps"][0]["name"], "dangling");
    assert_eq!(result["steps"][0]["status"], "broken");
    assert_eq!(result["steps"][0]["stage"], "finished");
}

#[test]
fn derives_history_id_after_scope_metadata_is_merged() {
    let (lifecycle, out_dir) = make_lifecycle("history-id-after-merge");

    lifecycle.start_test_case(StartTestCaseParams::new("display").with_full_name("pkg::display"));
    lifecycle.add_parameter("zeta", "1");
    let test_uuid = lifecycle
        .current_test_uuid()
        .expect("test uuid should exist");
    let scope_uuid = lifecycle.start_scope(Some("scope name".to_string()));
    lifecycle.link_scope_to_test(&scope_uuid, &test_uuid);
    lifecycle.start_before_fixture(&scope_uuid, "before fixture");
    lifecycle.stop_before_fixture(&scope_uuid, Status::Passed, None);

    {
        let mut lock = lifecycle
            .state
            .lock()
            .expect("lifecycle lock should be available");
        let scope = lock
            .scopes
            .get_mut(&scope_uuid)
            .expect("scope should still exist before write");
        scope.container.befores[0].parameters.push(Parameter {
            name: "alpha".to_string(),
            value: "2".to_string(),
            excluded: None,
            mode: None,
        });
        scope.container.befores[0].parameters.push(Parameter {
            name: "ignored".to_string(),
            value: "3".to_string(),
            excluded: Some(true),
            mode: None,
        });
    }

    lifecycle.stop_scope(&scope_uuid);
    lifecycle.stop_test_case(Status::Passed, None);

    let results = read_jsons_with_suffix(&out_dir, "-result.json");
    assert_eq!(results.len(), 1);
    let result = &results[0];
    let parameter_hash = md5_hex("alpha:2,zeta:1");
    let expected_history_id = md5_hex(&format!("{}:{parameter_hash}", md5_hex("pkg::display")));
    assert_eq!(result["testCaseId"], md5_hex("pkg::display"));
    assert_eq!(result["historyId"], expected_history_id);
}

#[test]
fn stop_paths_normalize_missing_timing_fields() {
    let (lifecycle, out_dir) = make_lifecycle("normalize-missing-timing");

    lifecycle.start_test_case("normalize");
    lifecycle.start_step("outer");
    lifecycle.start_step("inner");

    {
        let mut lock = lifecycle
            .state
            .lock()
            .expect("lifecycle lock should be available");
        let test_uuid = lifecycle
            .current_test_uuid()
            .expect("test uuid should exist");
        let test = lock
            .tests
            .get_mut(&test_uuid)
            .expect("test state should exist");
        test.test.start = None;
        test.test.stop = None;
        test.step_stack[0].start = None;
        test.step_stack[0].stop = None;
        test.step_stack[1].start = Some(500);
        test.step_stack[1].stop = None;
    }

    lifecycle.stop_test_case(Status::Passed, None);

    let results = read_jsons_with_suffix(&out_dir, "-result.json");
    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert!(result["start"].as_i64().is_some());
    assert!(result["stop"].as_i64().is_some());
    assert!(result["stop"].as_i64().unwrap() >= result["start"].as_i64().unwrap());
    assert!(result["steps"][0]["start"].as_i64().is_some());
    assert!(result["steps"][0]["stop"].as_i64().is_some());
    assert!(
        result["steps"][0]["stop"].as_i64().unwrap()
            >= result["steps"][0]["start"].as_i64().unwrap()
    );
    assert_eq!(result["steps"][0]["steps"][0]["start"], 500);
    assert!(result["steps"][0]["steps"][0]["stop"].as_i64().unwrap() >= 500);
}

#[test]
fn scope_and_fixture_public_methods_write_container_and_merge_metadata() {
    let (lifecycle, out_dir) = make_lifecycle("scope-and-fixtures");

    lifecycle.start_test_case("scoped-test");
    let test_uuid = lifecycle
        .current_test_uuid()
        .expect("test uuid should exist");
    let scope_uuid = lifecycle.start_scope(Some("scope name".to_string()));
    lifecycle.link_scope_to_test(&scope_uuid, &test_uuid);
    lifecycle.start_before_fixture(&scope_uuid, "before fixture");
    lifecycle.start_step("before step");
    lifecycle.add_attachment("before.txt", "text/plain", b"before");
    lifecycle.stop_step(Status::Passed, None);
    lifecycle.stop_before_fixture(&scope_uuid, Status::Passed, None);
    lifecycle.start_after_fixture(&scope_uuid, "after fixture");
    lifecycle.stop_after_fixture(&scope_uuid, Status::Passed, None);

    {
        let mut lock = lifecycle
            .state
            .lock()
            .expect("lifecycle lock should be available");
        let scope = lock
            .scopes
            .get_mut(&scope_uuid)
            .expect("scope should still exist before write");
        scope.container.links.push(Link {
            name: Some("scope-link".to_string()),
            url: "https://example.invalid/scope".to_string(),
            link_type: Some("issue".to_string()),
        });
        scope.container.befores[0].parameters.push(Parameter {
            name: "scope-param".to_string(),
            value: "42".to_string(),
            excluded: None,
            mode: None,
        });
    }

    lifecycle.stop_scope(&scope_uuid);
    lifecycle.stop_test_case(Status::Passed, None);
    lifecycle.write_scope(&scope_uuid);

    let containers = read_jsons_with_suffix(&out_dir, "-container.json");
    assert_eq!(containers.len(), 1);
    let container = &containers[0];
    assert_eq!(container["name"], "scope name");
    assert_eq!(container["children"][0], test_uuid);
    assert_eq!(container["befores"][0]["name"], "before fixture");
    assert_eq!(container["befores"][0]["steps"][0]["name"], "before step");
    assert_eq!(container["afters"][0]["name"], "after fixture");

    let results = read_jsons_with_suffix(&out_dir, "-result.json");
    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert_eq!(result["links"][0]["url"], "https://example.invalid/scope");
    assert_eq!(result["parameters"][0]["name"], "scope-param");
}

#[test]
fn stop_step_preserves_existing_runtime_status_and_metadata_operations_target_current_step() {
    let (lifecycle, out_dir) = make_lifecycle("step-metadata-and-preserve-status");

    lifecycle.start_test_case("metadata");
    lifecycle.start_step("original");
    lifecycle.set_current_step_display_name("renamed");
    lifecycle.add_current_step_parameter("key", "value");

    {
        let mut lock = lifecycle
            .state
            .lock()
            .expect("lifecycle lock should be available");
        let test_uuid = lifecycle
            .current_test_uuid()
            .expect("test uuid should exist");
        let test = lock
            .tests
            .get_mut(&test_uuid)
            .expect("test state should exist");
        test.step_stack[0].status = Some(Status::Broken);
        test.step_stack[0].status_details = Some(StatusDetails {
            message: Some("runtime status".to_string()),
            trace: None,
            actual: None,
            expected: None,
        });
    }

    lifecycle.stop_step(
        Status::Passed,
        Some(StatusDetails {
            message: Some("stop status".to_string()),
            trace: None,
            actual: None,
            expected: None,
        }),
    );
    lifecycle.stop_test_case(Status::Passed, None);

    let results = read_jsons_with_suffix(&out_dir, "-result.json");
    let step = &results[0]["steps"][0];
    assert_eq!(step["name"], "renamed");
    assert_eq!(step["parameters"][0]["name"], "key");
    assert_eq!(step["parameters"][0]["value"], "value");
    assert_eq!(step["status"], "broken");
    assert_eq!(step["statusDetails"]["message"], "runtime status");
}

#[test]
fn log_step_with_uses_same_start_and_stop_timestamp() {
    let (lifecycle, out_dir) = make_lifecycle("log-step-same-timestamp");
    let allure = crate::facade::AllureFacade::with_lifecycle(lifecycle.clone());

    lifecycle.start_test_case("log-step");
    allure.log_step_with("instant", None, None::<String>);
    lifecycle.stop_test_case(Status::Passed, None);

    let results = read_jsons_with_suffix(&out_dir, "-result.json");
    let step = &results[0]["steps"][0];
    assert_eq!(step["status"], "passed");
    assert_eq!(step["start"], step["stop"]);
}

#[test]
fn step_with_classifies_panic_and_rethrows_original_error() {
    let (lifecycle, out_dir) = make_lifecycle("step-with-classifies-panic");
    let allure = crate::facade::AllureFacade::with_lifecycle(lifecycle.clone());

    lifecycle.start_test_case("panic-step");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        allure.step_with("assertion step", || {
            panic!("assertion failed: expected true")
        });
    }));
    assert!(result.is_err());
    lifecycle.stop_test_case(Status::Passed, None);

    let results = read_jsons_with_suffix(&out_dir, "-result.json");
    let step = &results[0]["steps"][0];
    assert_eq!(step["name"], "assertion step");
    assert_eq!(step["status"], "failed");
    assert_eq!(
        step["statusDetails"]["message"],
        "assertion failed: expected true"
    );
}
