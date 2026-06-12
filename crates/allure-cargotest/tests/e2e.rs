use allure_rust_commons::{self as allure, md5_hex};

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static TEMP_PROJECT_COUNTER: AtomicU64 = AtomicU64::new(1);

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
        let counter = TEMP_PROJECT_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("{prefix}-{}-{nanos}-{counter}", std::process::id()));
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
#[allure_cargotest::log_asserts]
fn generates_allure_results_for_descriptions_sample() {
    allure::test(|| {
        allure::description(
            "Verifies the description sample emits markdown and HTML descriptions in the Allure result.",
        );
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
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn generates_allure_results_for_labels_sample() {
    allure::test(|| {
        allure::description(
            "Verifies the labels sample emits custom, hierarchy, owner, severity, tag, id, and link metadata.",
        );
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
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn generates_synthetic_suite_labels_when_not_overridden() {
    allure::test(|| {
        allure::description(
            "Verifies synthetic suite labels are generated when the sample does not override them.",
        );
        let (results, _, _project_dir) = run_sample("labels", true);
        let result = results
            .get("derives_synthetic_suite_labels_by_default")
            .expect("missing derives_synthetic_suite_labels_by_default result");

        assert_has_allure_result_fields(result);
        assert_eq!(json_string(result, "status"), Some("passed"));

        assert!(contains_label(result, "suite", "allure"));
        assert!(!contains_label_name(result, "parentSuite"));
        assert!(!contains_label_name(result, "subSuite"));
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn generates_allure_results_for_links_sample() {
    allure::test(|| {
        allure::description(
            "Verifies the links sample emits custom, issue, and TMS links in order.",
        );
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
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn generates_allure_results_for_parameters_sample() {
    allure::test(|| {
        allure::description(
            "Verifies the parameters sample emits runtime parameters with expected names and values.",
        );
        let (results, _, _project_dir) = run_sample("parameters", true);
        let result = results
            .get("writes_parameters")
            .expect("missing writes_parameters result");

        assert_has_allure_result_fields(result);
        assert_eq!(json_string(result, "status"), Some("passed"));
        assert!(result.contains(
            "\"parameters\":[{\"name\":\"browser\",\"value\":\"firefox\",\"excluded\":null,\"mode\":null}"
        ));
        assert!(result
            .contains("{\"name\":\"retries\",\"value\":\"2\",\"excluded\":null,\"mode\":null}]"));
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn generates_allure_results_for_attachments_sample() {
    allure::test(|| {
        allure::description(
            "Verifies the attachments sample writes attachment metadata and file content.",
        );
        let (results, results_dir, _project_dir) = run_sample("attachments", true);
        let result = results
            .get("writes_attachment")
            .expect("missing writes_attachment result");

        assert_has_allure_result_fields(result);
        assert_eq!(json_string(result, "status"), Some("passed"));
        assert!(result.contains("\"attachments\":[{\"name\":\"hello.txt\""));
        assert!(result.contains("\"type\":\"text/plain\""));

        let attachment_source =
            json_string(result, "source").expect("attachment source should exist");
        let attachment_path = results_dir.join(attachment_source);
        let attachment_content =
            fs::read_to_string(attachment_path).expect("attachment should exist");
        assert_eq!(attachment_content, "hello from attachments sample");
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn generates_allure_results_from_commons_runtime_functions() {
    allure::test(|| {
        allure::description(
            "Verifies cargotest preserves steps, parameters, labels, and attachments from commons runtime functions.",
        );
        let (results, results_dir, _project_dir) = run_sample("functions", true);
        let result = results
            .get("uses_commons_runtime_functions")
            .expect("missing uses_commons_runtime_functions result");

        assert_has_allure_result_fields(result);
        assert_eq!(json_string(result, "status"), Some("passed"));
        assert!(contains_label(result, "feature", "Runtime functions"));
        assert!(result.contains(
            "\"parameters\":[{\"name\":\"source\",\"value\":\"commons\",\"excluded\":null,\"mode\":null}]"
        ));
        assert!(
            result.contains("{\"uuid\":null,\"name\":\"log from commons\",\"status\":\"passed\"")
        );
        assert!(result
            .contains("{\"uuid\":null,\"name\":\"logged from commons\",\"status\":\"passed\""));
        assert!(result
            .contains("{\"uuid\":null,\"name\":\"attach from commons\",\"status\":\"passed\""));
        assert!(result.contains("{\"uuid\":null,\"name\":\"commons.txt\",\"status\":\"passed\""));
        assert!(result.contains("\"attachments\":[{\"name\":\"commons.txt\""));

        let attachment_source =
            json_string(result, "source").expect("attachment source should exist");
        let attachment_path = results_dir.join(attachment_source);
        let attachment_content =
            fs::read_to_string(attachment_path).expect("attachment should exist");
        assert_eq!(attachment_content, "attached from commons");
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn generates_allure_results_from_commons_test_runtime() {
    allure::test(|| {
        allure::description(
            "Verifies manual commons test runtime output is discovered and written without macro metadata.",
        );
        let (results, results_dir, _project_dir) = run_sample_without_title_path("runtime", true);
        let result = results
            .get("uses_commons_test_runtime")
            .expect("missing uses_commons_test_runtime result");

        assert_has_allure_result_fields(result);
        assert_eq!(json_string(result, "status"), Some("passed"));
        assert!(json_string(result, "fullName")
            .is_some_and(|full_name| full_name.ends_with("uses_commons_test_runtime")));
        assert!(contains_label(result, "feature", "Manual runtime"));
        assert!(result.contains(
            "\"parameters\":[{\"name\":\"style\",\"value\":\"no-macro\",\"excluded\":null,\"mode\":null}]"
        ));
        assert!(result
            .contains("{\"uuid\":null,\"name\":\"log from manual runtime\",\"status\":\"passed\""));
        assert!(result.contains(
            "{\"uuid\":null,\"name\":\"logged from manual runtime\",\"status\":\"passed\""
        ));
        assert!(result.contains(
            "{\"uuid\":null,\"name\":\"attach from manual runtime\",\"status\":\"passed\""
        ));
        assert!(result.contains("{\"uuid\":null,\"name\":\"runtime.txt\",\"status\":\"passed\""));
        assert!(result.contains("\"attachments\":[{\"name\":\"runtime.txt\""));

        let attachment_source =
            json_string(result, "source").expect("attachment source should exist");
        let attachment_path = results_dir.join(attachment_source);
        let attachment_content =
            fs::read_to_string(attachment_path).expect("attachment should exist");
        assert_eq!(attachment_content, "attached from manual runtime");
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn generates_allure_results_for_steps_sample() {
    allure::test(|| {
        allure::description(
            "Verifies the steps sample preserves passed, failed, broken, nested, and attached step evidence.",
        );
        let (results, results_dir, _project_dir) = run_sample("steps", true);
        let result = results
            .get("writes_steps")
            .expect("missing writes_steps result");

        assert_has_allure_result_fields(result);
        assert_eq!(json_string(result, "status"), Some("passed"));

        assert!(result
            .contains("\"steps\":[{\"uuid\":null,\"name\":\"simple step\",\"status\":\"passed\""));
        assert!(result.contains("{\"uuid\":null,\"name\":\"logged step\",\"status\":\"passed\""));
        assert!(result.contains(
            "{\"uuid\":null,\"name\":\"some_step_doing_something\",\"status\":\"passed\""
        ));
        assert!(result
            .contains("{\"uuid\":null,\"name\":\"Readable step title\",\"status\":\"passed\""));
        assert!(result.contains("{\"uuid\":null,\"name\":\"failed step\",\"status\":\"failed\""));
        assert!(result.contains("\"statusDetails\":{\"message\":\"step failed\""));
        assert!(result.contains("{\"uuid\":null,\"name\":\"broken step\",\"status\":\"broken\""));
        assert!(result.contains("\"statusDetails\":{\"message\":\"step broken\""));
        assert!(result.contains("{\"uuid\":null,\"name\":\"nested parent\",\"status\":\"passed\""));
        assert!(result.contains("{\"uuid\":null,\"name\":\"nested child\",\"status\":\"passed\""));
        assert!(result.contains("\"attachments\":[{\"name\":\"nested.txt\""));

        let attachment_source = result
            .split("\"name\":\"nested.txt\",\"source\":\"")
            .nth(1)
            .and_then(|tail| tail.split('"').next())
            .expect("nested step attachment source should exist");

        let attachment_content = fs::read_to_string(results_dir.join(attachment_source))
            .expect("nested attachment should exist");
        assert_eq!(attachment_content, "inside nested step");
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn generates_allure_results_for_tokio_async_tests() {
    allure::test(|| {
        allure::description(
            "Verifies async Tokio tests preserve metadata, awaited steps, and attachments across runtime flavors.",
        );
        let (results, results_dir, _project_dir) = run_sample("tokio_async", true);

        let current_thread = results
            .get("Async custom name")
            .expect("missing Async custom name result");
        assert_has_allure_result_fields(current_thread);
        assert_eq!(json_string(current_thread, "status"), Some("passed"));
        assert_eq!(
            json_string(current_thread, "fullName"),
            Some("allure::writes_tokio_async_metadata")
        );
        assert!(contains_label(current_thread, "ALLURE_ID", "ASYNC-1"));
        assert!(contains_label(
            current_thread,
            "component",
            "tokio-current-thread"
        ));
        assert!(current_thread.contains(
            "\"parameters\":[{\"name\":\"phase\",\"value\":\"after-await\",\"excluded\":null,\"mode\":null}]"
        ));
        assert!(current_thread.contains(
            "\"steps\":[{\"uuid\":null,\"name\":\"async helper step\",\"status\":\"passed\""
        ));
        assert!(current_thread.contains("\"attachments\":[{\"name\":\"async.txt\""));

        let attachment_source =
            json_string(current_thread, "source").expect("async attachment source should exist");
        let attachment_content = fs::read_to_string(results_dir.join(attachment_source))
            .expect("async attachment should exist");
        assert_eq!(attachment_content, "hello from async test");

        let multi_thread = results
            .get("writes_tokio_multi_thread_metadata_after_await")
            .expect("missing writes_tokio_multi_thread_metadata_after_await result");
        assert_has_allure_result_fields(multi_thread);
        assert_eq!(json_string(multi_thread, "status"), Some("passed"));
        assert!(contains_label(
            multi_thread,
            "component",
            "tokio-multi-thread"
        ));
        assert!(multi_thread.contains(
            "\"steps\":[{\"uuid\":null,\"name\":\"direct async stage\",\"status\":\"passed\""
        ));
        assert!(multi_thread
            .contains("{\"uuid\":null,\"name\":\"async helper step\",\"status\":\"passed\""));
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn generates_allure_results_for_failing_tests() {
    allure::test(|| {
        allure::description(
            "Verifies failing sample tests still emit both passed and failed Allure results.",
        );
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
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn generates_assertion_steps_by_default() {
    allure::test(|| {
        allure::description(
            "Verifies assertion logging is enabled by default and records passing and failing assertion details.",
        );
        let (results, _, _project_dir) = run_sample("assertions", false);

        let passing = results
            .get("logs_passing_assertions")
            .expect("missing logs_passing_assertions result");
        assert_has_allure_result_fields(passing);
        assert_eq!(json_string(passing, "status"), Some("passed"));
        assert!(passing.contains("\"name\":\"assert!(true)\",\"status\":\"passed\""));
        assert!(passing.contains("assert_eq!"));
        assert!(passing.contains("assert_ne!"));
        assert!(passing.contains("\"name\":\"step_assertions_are_nested\""));
        assert!(passing.contains("\"name\":\"assert_eq!(1 + 1, 2)\""));

        let failing = results
            .get("logs_failed_assertion_details")
            .expect("missing logs_failed_assertion_details result");
        assert_has_allure_result_fields(failing);
        assert_eq!(json_string(failing, "status"), Some("failed"));
        assert!(failing.contains("\"name\":\"assert_eq!"));
        assert!(failing.contains("\"status\":\"failed\""));
        assert!(failing.contains(r#""actual":"\"actual\"""#));
        assert!(failing.contains(r#""expected":"\"expected\"""#));
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn disables_assertion_steps_with_environment_override() {
    allure::test(|| {
        allure::description(
            "Verifies the ALLURE_LOG_ASSERTS environment override suppresses assertion step logging.",
        );
        let mut envs = HashMap::new();
        envs.insert("ALLURE_LOG_ASSERTS", "false");
        let (results, _, _project_dir) = run_sample_with_env("assertions", false, &envs);

        let passing = results
            .get("logs_passing_assertions")
            .expect("missing logs_passing_assertions result");
        assert_has_allure_result_fields(passing);
        assert_eq!(json_string(passing, "status"), Some("passed"));
        assert!(!passing.contains("\"name\":\"assert!(true)\""));
        assert!(!passing.contains("\"name\":\"assert_eq!"));
        assert!(!passing.contains("\"name\":\"assert_ne!"));

        let failing = results
            .get("logs_failed_assertion_details")
            .expect("missing logs_failed_assertion_details result");
        assert_has_allure_result_fields(failing);
        assert_eq!(json_string(failing, "status"), Some("failed"));
        assert!(!failing.contains("\"name\":\"assert_eq!"));
        assert!(!failing.contains(r#""actual":"\"actual\"""#));
        assert!(!failing.contains(r#""expected":"\"expected\"""#));
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn generates_allure_results_for_should_panic_tests() {
    allure::test(|| {
        allure::description(
            "Verifies should-panic tests map matching, mismatching, and missing panics to the correct Allure status.",
        );
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
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn generates_allure_results_for_tokio_async_should_panic_tests() {
    allure::test(|| {
        allure::description(
            "Verifies async should-panic tests preserve panic status semantics under Tokio.",
        );
        let (results, _, _project_dir) = run_sample("tokio_async_should_panic", false);

        let without_expected = results
            .get("tokio_should_panic_without_expected_passes")
            .expect("missing tokio_should_panic_without_expected_passes result");
        assert_has_allure_result_fields(without_expected);
        assert_eq!(json_string(without_expected, "status"), Some("passed"));

        let with_expected = results
            .get("tokio_should_panic_with_expected_passes")
            .expect("missing tokio_should_panic_with_expected_passes result");
        assert_has_allure_result_fields(with_expected);
        assert_eq!(json_string(with_expected, "status"), Some("passed"));

        let expected_mismatch = results
            .get("tokio_should_panic_with_expected_mismatch_fails")
            .expect("missing tokio_should_panic_with_expected_mismatch_fails result");
        assert_has_allure_result_fields(expected_mismatch);
        assert_eq!(json_string(expected_mismatch, "status"), Some("failed"));
        assert!(expected_mismatch.contains("panic message mismatch: expected substring"));
        assert!(expected_mismatch.contains("needle"));
        assert!(expected_mismatch.contains("different async panic message"));

        let without_panic = results
            .get("tokio_should_panic_without_panic_fails")
            .expect("missing tokio_should_panic_without_panic_fails result");
        assert_has_allure_result_fields(without_panic);
        assert_eq!(json_string(without_panic, "status"), Some("failed"));
        assert_eq!(
            json_string(without_panic, "message"),
            Some("expected panic but none occurred")
        );
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn generates_default_runtime_labels() {
    allure::test(|| {
        allure::description(
            "Verifies default runtime labels and package metadata labels are emitted for sample tests.",
        );
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
        assert!(contains_label(
            result,
            "module",
            "allure-cargotest-sample-default_and_global_labels"
        ));
        assert!(contains_label(result, "component", "sample-fixture"));
        assert!(contains_label(result, "a", "a-value"));
        assert!(contains_label(result, "b", "b-value1"));
        assert!(contains_label(result, "b", "b-value2"));
        assert!(contains_label(result, "layer", "module-config"));
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn generates_global_labels_and_runtime_overrides() {
    allure::test(|| {
        allure::description(
            "Verifies environment-provided global labels override host, thread, component, and layer metadata.",
        );
        let mut envs = HashMap::new();
        envs.insert("ALLURE_LABEL_component", "checkout");
        envs.insert("allure.label.layer", "e2e");
        envs.insert("ALLURE_HOST_NAME", "ci-host");
        envs.insert("ALLURE_THREAD_NAME", "worker-7");

        let (results, _, _project_dir) =
            run_sample_with_env("default_and_global_labels", true, &envs);
        let result = results
            .get("writes_default_and_global_labels")
            .expect("missing writes_default_and_global_labels result");

        assert_has_allure_result_fields(result);
        assert_eq!(json_string(result, "status"), Some("passed"));

        assert!(contains_label(result, "host", "ci-host"));
        assert!(contains_label(result, "thread", "worker-7"));
        assert!(contains_label(result, "component", "checkout"));
        assert!(contains_label(result, "layer", "e2e"));
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn selective_run_when_test_selected_by_name() {
    allure::test(|| {
        allure::description(
            "Verifies test-plan selection by exact full name runs the selected sample test.",
        );
        let testplan = r#"{"version":"1.0","tests":[{"selector":"allure::selected_by_name"}]}"#;
        let (results, _, _project_dir) =
            run_sample_with_testplan("selective", Some(testplan), true);

        assert_eq!(results.len(), 1);
        assert!(results.contains_key("selected_by_name"));
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn selective_run_does_not_match_partial_selector_name() {
    allure::test(|| {
        allure::description(
            "Verifies partial selector names do not accidentally select sample tests.",
        );
        let testplan = r#"{"version":"1.0","tests":[{"selector":"selected_by_name"}]}"#;
        let (results, _, _project_dir) =
            run_sample_with_testplan("selective", Some(testplan), true);

        assert!(results.is_empty());
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn generates_allure_id_label_from_macro_attribute() {
    allure::test(|| {
        allure::description("Verifies macro-provided Allure ids are emitted as result labels.");
        let (results, _, _project_dir) = run_sample("selective", true);

        let result = results
            .get("selected_by_id")
            .expect("missing selected_by_id result");
        assert!(contains_label(result, "ALLURE_ID", "A-2"));
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn selective_run_when_multiple_tests_selected() {
    allure::test(|| {
        allure::description(
            "Verifies a test plan with multiple entries still emits only runnable selected sample tests.",
        );
        let testplan =
            r#"{"version":"1.0","tests":[{"selector":"allure::selected_by_name"},{"id":"A-2"}]}"#;
        let (results, _, _project_dir) =
            run_sample_with_testplan("selective", Some(testplan), true);

        assert_eq!(results.len(), 1);
        assert!(results.contains_key("selected_by_name"));
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn selective_run_when_no_tests_selected() {
    allure::test(|| {
        allure::description(
            "Verifies a test plan with no matching entries produces no sample Allure results.",
        );
        let testplan = r#"{"version":"1.0","tests":[{"selector":"missing::test"}]}"#;
        let (results, _, _project_dir) =
            run_sample_with_testplan("selective", Some(testplan), true);

        assert!(results.is_empty());
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn selective_run_with_malformed_testplan() {
    allure::test(|| {
        allure::description(
            "Verifies malformed test plans fall back to running the full sample suite.",
        );
        let (results, _, _project_dir) =
            run_sample_with_testplan("selective", Some("not json"), true);

        assert_eq!(results.len(), 3);
        assert!(results.contains_key("selected_by_name"));
        assert!(results.contains_key("selected_by_id"));
        assert!(results.contains_key("selected_extra"));
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn selective_run_with_missing_testplan_file_and_env_var() {
    allure::test(|| {
        allure::description(
            "Verifies a missing test-plan file falls back to running the full sample suite.",
        );
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
    });
}

#[test]
#[allure_cargotest::log_asserts]
fn keeps_metadata_linked_to_the_right_test_when_running_concurrently() {
    allure::test(|| {
        allure::description(
            "Verifies concurrent sample tests keep labels, parameters, and links isolated per test.",
        );
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
    });
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
        attach_testplan_input(sample_name, content);
        plan_path = Some(path);
    }

    if let Some(path) = &plan_path {
        envs.insert(
            "ALLURE_TESTPLAN_PATH",
            path.to_str().expect("path should be utf-8"),
        );
    }

    let output = run_cargo_test(project_dir.path(), &results_dir, &envs);
    attach_command_output(sample_name, &output);
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

    let results = read_results_by_test_name_allow_empty(&results_dir);
    attach_result_summary(sample_name, &results, &results_dir);
    assert_results_title_path(&results, &["tests", "allure.rs"]);

    (results, results_dir, project_dir)
}

#[test]
#[allure_cargotest::log_asserts]
fn generates_test_case_id_and_runtime_override() {
    allure::test(|| {
        allure::description(
            "Verifies generated testCaseId values are stable and can be overridden at runtime.",
        );
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
    });
}

fn run_sample(
    sample_name: &str,
    expect_success: bool,
) -> (HashMap<String, String>, PathBuf, TempProjectDir) {
    run_sample_with_env(sample_name, expect_success, &HashMap::new())
}

fn run_sample_without_title_path(
    sample_name: &str,
    expect_success: bool,
) -> (HashMap<String, String>, PathBuf, TempProjectDir) {
    let (results, results_dir, project_dir) =
        run_sample_with_env_and_title_path(sample_name, expect_success, &HashMap::new(), None);

    assert!(
        !results.is_empty(),
        "no result files found in {}",
        results_dir.display()
    );

    (results, results_dir, project_dir)
}

fn run_sample_with_env_allow_empty(
    sample_name: &str,
    expect_success: bool,
    envs: &HashMap<&str, &str>,
) -> (HashMap<String, String>, PathBuf, TempProjectDir) {
    run_sample_with_env_and_title_path(
        sample_name,
        expect_success,
        envs,
        Some(&["tests", "allure.rs"][..]),
    )
}

fn run_sample_with_env_and_title_path(
    sample_name: &str,
    expect_success: bool,
    envs: &HashMap<&str, &str>,
    expected_title_path: Option<&[&str]>,
) -> (HashMap<String, String>, PathBuf, TempProjectDir) {
    let project_dir = prepare_sample_project(sample_name);
    let results_dir = project_dir.path().join("allure-results");

    let output = run_cargo_test(project_dir.path(), &results_dir, envs);
    attach_command_output(sample_name, &output);
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

    assert!(
        results_dir.exists(),
        "no result files found for {sample_name}; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let results = read_results_by_test_name_allow_empty(&results_dir);
    attach_result_summary(sample_name, &results, &results_dir);
    if let Some(expected_title_path) = expected_title_path {
        assert_results_title_path(&results, expected_title_path);
    }

    (results, results_dir, project_dir)
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
    let samples_root = repo_root.join("smokes").join("allure-cargotest");

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

    let extra_dependencies = if sample_name.starts_with("tokio_async") {
        r#"tokio = { version = "1", features = ["macros", "rt", "rt-multi-thread", "time"] }
"#
    } else {
        ""
    };
    let allure_cargotest_path = toml_string(&repo_root.join("crates").join("allure-cargotest"));
    let allure_rust_commons_path =
        toml_string(&repo_root.join("crates").join("allure-rust-commons"));
    let cargo_toml = format!(
        r#"[package]
name = "allure-cargotest-sample-{}"
version = "0.1.0"
edition = "2021"

[workspace]

[dependencies]
allure-cargotest = {{ path = {} }}
allure-rust-commons = {{ path = {} }}
{}

[package.metadata.allure.labels]
module = "allure-cargotest-sample-{}"

[[package.metadata.allure.modules]]
path = "tests/allure.rs"
labels = {{ component = "sample-fixture", a = "a-value", b = ["b-value1", "b-value2"] }}

[[package.metadata.allure.modules]]
module = "allure"
labels = {{ layer = "module-config" }}
"#,
        sample_name,
        allure_cargotest_path,
        allure_rust_commons_path,
        extra_dependencies,
        sample_name
    );
    fs::write(project_dir.path().join("Cargo.toml"), cargo_toml)
        .expect("sample Cargo.toml should be generated");

    project_dir
}

fn toml_string(path: &Path) -> String {
    let value = path
        .to_str()
        .expect("path should be utf-8")
        .replace('\\', "/");
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch => output.push(ch),
        }
    }
    output.push('"');
    output
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

fn attach_testplan_input(sample_name: &str, content: &str) {
    allure::attachment(
        format!("{sample_name} testplan input"),
        "text/plain",
        content.as_bytes(),
    );
}

fn attach_command_output(sample_name: &str, output: &std::process::Output) {
    let status = format!(
        "status: {}\ncode: {:?}\nsuccess: {}\n",
        output.status,
        output.status.code(),
        output.status.success()
    );
    allure::attachment(
        format!("{sample_name} cargo test status"),
        "text/plain",
        status.as_bytes(),
    );

    if !output.stdout.is_empty() {
        allure::attachment(
            format!("{sample_name} cargo test stdout"),
            "text/plain",
            &output.stdout,
        );
    }

    if !output.stderr.is_empty() {
        allure::attachment(
            format!("{sample_name} cargo test stderr"),
            "text/plain",
            &output.stderr,
        );
    }
}

fn attach_result_summary(sample_name: &str, results: &HashMap<String, String>, results_dir: &Path) {
    let mut test_names = results.keys().collect::<Vec<_>>();
    test_names.sort();

    let mut summary = format!(
        "sample: {sample_name}\nresults_dir: {}\nresult_count: {}\n",
        results_dir.display(),
        test_names.len()
    );
    if test_names.is_empty() {
        summary.push_str("tests: <none>\n");
    } else {
        summary.push_str("tests:\n");
        for name in &test_names {
            summary.push_str("- ");
            summary.push_str(name);
            summary.push('\n');
        }
    }

    allure::attachment(
        format!("{sample_name} allure result summary"),
        "text/plain",
        summary.as_bytes(),
    );

    for name in test_names {
        let raw = results
            .get(name.as_str())
            .expect("result summary keys should match result map");
        allure::attachment(
            format!("{sample_name} result {name}.json"),
            "application/json",
            raw.as_bytes(),
        );
    }
}

fn read_results_by_test_name_allow_empty(results_dir: &Path) -> HashMap<String, String> {
    let mut parsed = HashMap::new();
    let mut result_files = Vec::new();

    if !results_dir.exists() {
        return parsed;
    }

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

fn assert_results_title_path(results: &HashMap<String, String>, expected: &[&str]) {
    for (name, result) in results {
        assert_title_path(result, expected, name);
    }
}

fn assert_title_path(result: &str, expected: &[&str], context: &str) {
    let expected_json = format!(
        "\"titlePath\":[{}]",
        expected
            .iter()
            .map(|part| format!("\"{part}\""))
            .collect::<Vec<_>>()
            .join(",")
    );
    assert!(
        result.contains(&expected_json),
        "expected titlePath {expected_json} for {context}: {result}"
    );
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
        "titlePath",
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
