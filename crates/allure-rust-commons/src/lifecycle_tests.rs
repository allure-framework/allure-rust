use super::*;
use crate::{
    http_exchange::{HttpExchange, HTTP_EXCHANGE_ATTACHMENT_MIME},
    test_utils::allure_test,
};
use std::{fs, path::PathBuf};

fn reset_active_roots() {
    ACTIVE_TEST_ROOT.with(|cell| cell.borrow_mut().clear());
    ACTIVE_SCOPE_ROOT.with(|cell| *cell.borrow_mut() = None);
}

fn make_lifecycle(test_name: &str) -> (AllureLifecycle, PathBuf) {
    reset_active_roots();
    make_lifecycle_without_reset(test_name)
}

fn make_lifecycle_without_reset(test_name: &str) -> (AllureLifecycle, PathBuf) {
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

fn contains_label(result: &serde_json::Value, name: &str, value: &str) -> bool {
    result["labels"]
        .as_array()
        .expect("labels should be an array")
        .iter()
        .any(|label| label["name"] == name && label["value"] == value)
}

#[test]
fn test_case_public_methods_are_persisted() {
    allure_test(
        module_path!(),
        "test_case_public_methods_are_persisted",
        || {
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
            assert!(contains_label(result, "suite", "commons"));
            assert_eq!(result["links"][0]["url"], "https://example.invalid/case");
            assert_eq!(result["parameters"][0]["name"], "browser");
            assert_eq!(result["steps"][0]["name"], "root step");
            assert_eq!(result["steps"][0]["status"], "passed");
        },
    );
}

#[test]
fn add_http_exchange_writes_httpexchange_attachment() {
    allure_test(
        module_path!(),
        "add_http_exchange_writes_httpexchange_attachment",
        || {
            let (lifecycle, out_dir) = make_lifecycle("http-exchange-attachment");

            lifecycle.start_test_case("http-test");
            lifecycle.add_http_exchange(HttpExchange::new(
                "GET",
                "https://api.example.com/v1/orders/42",
            ));
            lifecycle.stop_test_case(Status::Passed, None);

            let results = read_jsons_with_suffix(&out_dir, "-result.json");
            assert_eq!(results.len(), 1);
            let attachment = &results[0]["attachments"][0];
            assert_eq!(attachment["name"], "HTTP Exchange");
            assert_eq!(attachment["type"], HTTP_EXCHANGE_ATTACHMENT_MIME);
            let source = attachment["source"]
                .as_str()
                .expect("attachment source should be a string");
            assert!(source.ends_with("-attachment.httpexchange"));

            let payload =
                fs::read_to_string(out_dir.join(source)).expect("attachment should be readable");
            let payload = serde_json::from_str::<serde_json::Value>(&payload)
                .expect("attachment should be json");
            assert_eq!(payload["schemaVersion"], 1);
            assert_eq!(payload["request"]["method"], "GET");
            assert_eq!(
                payload["request"]["url"],
                "https://api.example.com/v1/orders/42"
            );
        },
    );
}

#[test]
fn nested_test_contexts_restore_outer_active_test() {
    allure_test(
        module_path!(),
        "nested_test_contexts_restore_outer_active_test",
        || {
            let (outer, outer_dir) = make_lifecycle("nested-outer");

            outer.start_test_case("outer");
            let outer_uuid = outer
                .current_test_uuid()
                .expect("outer test uuid should be active");

            let (inner, inner_dir) = make_lifecycle_without_reset("nested-inner");
            inner.start_test_case("inner");
            assert_ne!(
                inner.current_test_uuid().as_deref(),
                Some(outer_uuid.as_str())
            );
            inner.stop_test_case(Status::Passed, None);

            assert_eq!(
                outer.current_test_uuid().as_deref(),
                Some(outer_uuid.as_str())
            );
            outer.add_parameter("after-inner", "true");
            outer.stop_test_case(Status::Passed, None);

            let inner_results = read_jsons_with_suffix(&inner_dir, "-result.json");
            assert_eq!(inner_results.len(), 1);
            assert_eq!(inner_results[0]["name"], "inner");
            assert_eq!(inner_results[0]["status"], "passed");

            let outer_results = read_jsons_with_suffix(&outer_dir, "-result.json");
            assert_eq!(outer_results.len(), 1);
            assert_eq!(outer_results[0]["name"], "outer");
            assert_eq!(outer_results[0]["status"], "passed");
            assert_eq!(outer_results[0]["parameters"][0]["name"], "after-inner");
        },
    );
}

#[test]
fn facade_http_exchange_named_wraps_attachment_in_ordered_step() {
    allure_test(
        module_path!(),
        "facade_http_exchange_named_wraps_attachment_in_ordered_step",
        || {
            let (lifecycle, out_dir) = make_lifecycle("facade-http-exchange-step");
            let allure = crate::facade::AllureFacade::with_lifecycle(lifecycle.clone());

            lifecycle.start_test_case("http-step-test");
            lifecycle.start_step("call api");
            allure.http_exchange_named(
                "Create order",
                HttpExchange::new("POST", "https://api.example.com/v1/orders"),
            );
            lifecycle.stop_step(Status::Passed, None);
            lifecycle.stop_test_case(Status::Passed, None);

            let results = read_jsons_with_suffix(&out_dir, "-result.json");
            assert_eq!(results.len(), 1);
            assert!(results[0]["attachments"]
                .as_array()
                .expect("root attachments should be an array")
                .is_empty());
            assert_eq!(results[0]["steps"][0]["name"], "call api");
            assert!(results[0]["steps"][0]["attachments"]
                .as_array()
                .expect("active step attachments should be an array")
                .is_empty());
            assert_eq!(results[0]["steps"][0]["steps"][0]["name"], "Create order");
            let attachment = &results[0]["steps"][0]["steps"][0]["attachments"][0];
            assert_eq!(results[0]["steps"][0]["steps"][0]["status"], "passed");
            assert_eq!(attachment["name"], "Create order");
            assert_eq!(attachment["type"], HTTP_EXCHANGE_ATTACHMENT_MIME);
            let source = attachment["source"]
                .as_str()
                .expect("attachment source should be a string");
            assert!(source.ends_with("-attachment.httpexchange"));
        },
    );
}

#[test]
fn reporter_http_exchange_keeps_exact_active_step_owner() {
    allure_test(
        module_path!(),
        "reporter_http_exchange_keeps_exact_active_step_owner",
        || {
            let (lifecycle, out_dir) = make_lifecycle("reporter-http-exchange-step");
            let allure = crate::facade::AllureFacade::with_lifecycle(lifecycle.clone());

            lifecycle.start_test_case("http-step-test");
            lifecycle.start_step("call api");
            crate::reporter::http_exchange_named(
                &allure,
                "Create order",
                HttpExchange::new("POST", "https://api.example.com/v1/orders"),
            );
            lifecycle.stop_step(Status::Passed, None);
            lifecycle.stop_test_case(Status::Passed, None);

            let results = read_jsons_with_suffix(&out_dir, "-result.json");
            let attachment = &results[0]["steps"][0]["attachments"][0];
            assert_eq!(results[0]["steps"][0]["name"], "call api");
            assert_eq!(attachment["name"], "Create order");
            assert_eq!(attachment["type"], HTTP_EXCHANGE_ATTACHMENT_MIME);
        },
    );
}

#[test]
fn facade_metadata_parameters_and_wrapped_file_attachments_are_persisted() {
    allure_test(
        module_path!(),
        "facade_metadata_parameters_and_wrapped_file_attachments_are_persisted",
        || {
            let (lifecycle, out_dir) = make_lifecycle("facade-reference-surface");
            let allure = crate::facade::AllureFacade::with_lifecycle(lifecycle.clone());
            let request_path = out_dir.join("request-source.json");
            let trace_path = out_dir.join("trace-source.zip");
            fs::write(&request_path, br#"{"ok":true}"#).expect("request source should be writable");
            fs::write(&trace_path, b"trace bytes").expect("trace source should be writable");

            lifecycle.start_test_case(
                StartTestCaseParams::new("original").with_full_name("pkg::original"),
            );
            allure.display_name("renamed test");
            allure.history_id("explicit-history");
            allure.test_case_id("explicit-case");
            allure.allure_id("A-1");
            allure.parameter_with_options(
                "secret",
                "value",
                Some(true),
                Some(ParameterMode::Masked),
            );
            allure.attachment("inline.json", "application/json", br#"{"inline":true}"#);
            allure
                .attachment_path("from-path.json", "application/json", &request_path)
                .expect("path attachment should be recorded");
            allure
                .attach_trace_named("session-trace.zip", &trace_path)
                .expect("trace attachment should be recorded");
            lifecycle.stop_test_case(Status::Passed, None);

            let results = read_jsons_with_suffix(&out_dir, "-result.json");
            assert_eq!(results.len(), 1);
            let result = &results[0];
            assert_eq!(result["name"], "renamed test");
            assert_eq!(result["fullName"], "pkg::original");
            assert_eq!(result["historyId"], "explicit-history");
            assert_eq!(result["testCaseId"], "explicit-case");
            assert!(contains_label(result, "ALLURE_ID", "A-1"));
            assert_eq!(result["parameters"][0]["name"], "secret");
            assert_eq!(result["parameters"][0]["excluded"], true);
            assert_eq!(result["parameters"][0]["mode"], "masked");
            assert!(result["attachments"]
                .as_array()
                .expect("root attachments should be an array")
                .is_empty());

            let steps = result["steps"]
                .as_array()
                .expect("wrapped attachments should be steps");
            assert_eq!(steps[0]["name"], "inline.json");
            assert_eq!(steps[0]["attachments"][0]["name"], "inline.json");
            assert_eq!(steps[0]["attachments"][0]["type"], "application/json");
            assert_eq!(steps[1]["name"], "from-path.json");
            assert_eq!(steps[1]["attachments"][0]["name"], "from-path.json");
            assert_eq!(steps[2]["name"], "session-trace.zip");
            assert_eq!(
                steps[2]["attachments"][0]["type"],
                crate::PLAYWRIGHT_TRACE_ATTACHMENT_MIME
            );
            let trace_source = steps[2]["attachments"][0]["source"]
                .as_str()
                .expect("trace source should be a string");
            assert!(trace_source.ends_with("-attachment.zip"));
        },
    );
}

#[test]
fn facade_global_diagnostics_use_lifecycle_writer() {
    allure_test(
        module_path!(),
        "facade_global_diagnostics_use_lifecycle_writer",
        || {
            let (lifecycle, out_dir) = make_lifecycle("facade-global-diagnostics");
            let allure = crate::facade::AllureFacade::with_lifecycle(lifecycle);

            allure
                .global_attachment("runner.log", "text/plain", b"runner output")
                .expect("global attachment should be recorded");
            allure
                .global_error_with_trace("runner failed", "stack trace")
                .expect("global error should be recorded");

            let globals = read_jsons_with_suffix(&out_dir, "-globals.json");
            assert_eq!(globals.len(), 2);
            let attachment_globals = globals
                .iter()
                .find(|value| {
                    value["attachments"]
                        .as_array()
                        .is_some_and(|attachments| !attachments.is_empty())
                })
                .expect("global attachment file should exist");
            let attachment = &attachment_globals["attachments"][0];
            assert_eq!(attachment["name"], "runner.log");
            assert_eq!(attachment["contentType"], "text/plain");
            let source = attachment["source"]
                .as_str()
                .expect("global attachment source should be a string");
            assert_eq!(
                fs::read_to_string(out_dir.join(source))
                    .expect("global attachment body should be readable"),
                "runner output"
            );

            let error_globals = globals
                .iter()
                .find(|value| {
                    value["errors"]
                        .as_array()
                        .is_some_and(|errors| !errors.is_empty())
                })
                .expect("global error file should exist");
            assert_eq!(error_globals["errors"][0]["message"], "runner failed");
            assert_eq!(error_globals["errors"][0]["trace"], "stack trace");
        },
    );
}

#[test]
fn start_test_case_accepts_optional_test_result_fields() {
    allure_test(
        module_path!(),
        "start_test_case_accepts_optional_test_result_fields",
        || {
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
            assert_eq!(result["historyId"], "custom-history");
            assert_eq!(result["testCaseId"], "custom-case");
            assert_eq!(result["description"], "markdown");
            assert_eq!(result["descriptionHtml"], "<p>html</p>");
            assert_eq!(result["status"], "passed");
            assert!(contains_label(result, "suite", "commons"));
            assert_eq!(result["links"][0]["name"], "docs");
            assert_eq!(result["parameters"][0]["name"], "browser");
            assert_eq!(result["steps"][0]["name"], "seed step");
            assert_eq!(result["attachments"][0]["name"], "seed.txt");
            assert_eq!(result["titlePath"][0], "module");
            assert_eq!(result["start"], 100);
            assert_eq!(result["stop"], 200);
        },
    );
}

#[test]
fn start_with_full_name_derives_test_case_id_and_finalizes_dangling_steps() {
    allure_test(
        module_path!(),
        "start_with_full_name_derives_test_case_id_and_finalizes_dangling_steps",
        || {
            let (lifecycle, out_dir) = make_lifecycle("full-name-and-finalize");

            lifecycle.start_test_case(
                StartTestCaseParams::new("display").with_full_name("pkg::display"),
            );
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
        },
    );
}

#[test]
fn derives_history_id_after_scope_metadata_is_merged() {
    allure_test(
        module_path!(),
        "derives_history_id_after_scope_metadata_is_merged",
        || {
            let (lifecycle, out_dir) = make_lifecycle("history-id-after-merge");

            lifecycle.start_test_case(
                StartTestCaseParams::new("display").with_full_name("pkg::display"),
            );
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
            let expected_history_id =
                md5_hex(&format!("{}:{parameter_hash}", md5_hex("pkg::display")));
            assert_eq!(result["testCaseId"], md5_hex("pkg::display"));
            assert_eq!(result["historyId"], expected_history_id);
        },
    );
}

#[test]
fn stop_paths_normalize_missing_timing_fields() {
    allure_test(
        module_path!(),
        "stop_paths_normalize_missing_timing_fields",
        || {
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
        },
    );
}

#[test]
fn scope_and_fixture_public_methods_write_container_and_merge_metadata() {
    allure_test(
        module_path!(),
        "scope_and_fixture_public_methods_write_container_and_merge_metadata",
        || {
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
        },
    );
}

#[test]
fn stop_step_preserves_existing_runtime_status_and_metadata_operations_target_current_step() {
    allure_test(
        module_path!(),
        "stop_step_preserves_existing_runtime_status_and_metadata_operations_target_current_step",
        || {
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
        },
    );
}

#[test]
fn log_step_with_uses_same_start_and_stop_timestamp() {
    allure_test(
        module_path!(),
        "log_step_with_uses_same_start_and_stop_timestamp",
        || {
            let (lifecycle, out_dir) = make_lifecycle("log-step-same-timestamp");
            let allure = crate::facade::AllureFacade::with_lifecycle(lifecycle.clone());

            lifecycle.start_test_case("log-step");
            allure.log_step_with("instant", None, None::<String>);
            lifecycle.stop_test_case(Status::Passed, None);

            let results = read_jsons_with_suffix(&out_dir, "-result.json");
            let step = &results[0]["steps"][0];
            assert_eq!(step["status"], "passed");
            assert_eq!(step["start"], step["stop"]);
        },
    );
}

#[test]
fn runtime_stage_boundaries_create_sibling_steps_and_nest_runtime_steps() {
    allure_test(
        module_path!(),
        "runtime_stage_boundaries_create_sibling_steps_and_nest_runtime_steps",
        || {
            let (lifecycle, out_dir) = make_lifecycle("runtime-stage-siblings");
            let allure = crate::facade::AllureFacade::with_lifecycle(lifecycle.clone());

            lifecycle.start_test_case("staged-test");
            allure.stage("prepare data");
            allure.log_step("created customer");
            allure.stage("submit order");
            allure.step("post order", || {
                allure.log_step("request sent");
            });
            allure.stage("verify result");
            allure.log_step("status is created");
            lifecycle.stop_test_case(Status::Passed, None);

            let results = read_jsons_with_suffix(&out_dir, "-result.json");
            let steps = results[0]["steps"]
                .as_array()
                .expect("steps should be serialized");
            assert_eq!(steps.len(), 3);
            assert_eq!(steps[0]["name"], "prepare data");
            assert_eq!(steps[0]["status"], "passed");
            assert_eq!(steps[0]["steps"][0]["name"], "created customer");
            assert_eq!(steps[1]["name"], "submit order");
            assert_eq!(steps[1]["steps"][0]["name"], "post order");
            assert_eq!(steps[1]["steps"][0]["steps"][0]["name"], "request sent");
            assert_eq!(steps[2]["name"], "verify result");
            assert_eq!(steps[2]["steps"][0]["name"], "status is created");
        },
    );
}

#[test]
fn runtime_stage_inside_wrapping_step_is_nested_under_that_step() {
    allure_test(
        module_path!(),
        "runtime_stage_inside_wrapping_step_is_nested_under_that_step",
        || {
            let (lifecycle, out_dir) = make_lifecycle("runtime-stage-inside-step");
            let allure = crate::facade::AllureFacade::with_lifecycle(lifecycle.clone());

            lifecycle.start_test_case("nested-stage-test");
            allure.step("parent step", || {
                allure.stage("prepare child");
                allure.log_step("nested runtime step");
                allure.stage("verify child");
            });
            lifecycle.stop_test_case(Status::Passed, None);

            let results = read_jsons_with_suffix(&out_dir, "-result.json");
            let parent = &results[0]["steps"][0];
            assert_eq!(parent["name"], "parent step");
            assert_eq!(parent["status"], "passed");
            assert_eq!(parent["steps"][0]["name"], "prepare child");
            assert_eq!(
                parent["steps"][0]["steps"][0]["name"],
                "nested runtime step"
            );
            assert_eq!(parent["steps"][1]["name"], "verify child");
            assert_eq!(parent["steps"][1]["status"], "passed");
        },
    );
}

#[test]
fn runtime_stage_inherits_enclosing_failure_status() {
    allure_test(
        module_path!(),
        "runtime_stage_inherits_enclosing_failure_status",
        || {
            let (lifecycle, out_dir) = make_lifecycle("runtime-stage-failure");
            let allure = crate::facade::AllureFacade::with_lifecycle(lifecycle.clone());

            lifecycle.start_test_case("failed-stage-test");
            allure.stage("verify result");
            lifecycle.stop_test_case(
                Status::Failed,
                Some(StatusDetails {
                    message: Some("expected created".to_string()),
                    trace: None,
                    actual: Some("pending".to_string()),
                    expected: Some("created".to_string()),
                }),
            );

            let results = read_jsons_with_suffix(&out_dir, "-result.json");
            let stage = &results[0]["steps"][0];
            assert_eq!(stage["name"], "verify result");
            assert_eq!(stage["status"], "failed");
            assert_eq!(stage["statusDetails"]["message"], "expected created");
            assert_eq!(stage["statusDetails"]["actual"], "pending");
            assert_eq!(stage["statusDetails"]["expected"], "created");
        },
    );
}

#[test]
fn step_classifies_panic_and_rethrows_original_error() {
    allure_test(
        module_path!(),
        "step_classifies_panic_and_rethrows_original_error",
        || {
            let (lifecycle, out_dir) = make_lifecycle("step-with-classifies-panic");
            let allure = crate::facade::AllureFacade::with_lifecycle(lifecycle.clone());

            lifecycle.start_test_case("panic-step");
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                allure.step("assertion step", || {
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
        },
    );
}
