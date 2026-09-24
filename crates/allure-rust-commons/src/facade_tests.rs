use super::*;
use crate::test_utils::allure_test;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::{Uuid, Version};

fn make_facade(test_name: &str) -> (AllureFacade, PathBuf) {
    let out_dir = std::env::temp_dir().join(format!(
        "allure-rust-facade-tests-{test_name}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos()
    ));
    let runtime = AllureRuntime::new(
        FileSystemResultsWriter::new(&out_dir).expect("writer should initialize"),
    );
    (AllureFacade::with_lifecycle(runtime.lifecycle()), out_dir)
}

fn read_result(out_dir: &PathBuf) -> serde_json::Value {
    let path = fs::read_dir(out_dir)
        .expect("results dir should exist")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.ends_with("-result.json"))
                .unwrap_or(false)
        })
        .expect("a result json should exist");
    serde_json::from_str(&fs::read_to_string(path).expect("result json should be readable"))
        .expect("result json should parse")
}

#[test]
fn global_attachment_without_lifecycle_uses_uuid_v4() {
    allure_test(
        module_path!(),
        "global_attachment_without_lifecycle_uses_uuid_v4",
        "Verifies an unbound facade writes a global attachment with a UUIDv4 source and preserves its contents.",
        || {
            let name = format!("runner-{}.log", Uuid::new_v4());
            step("Write a global attachment without a lifecycle", || {
                AllureFacade::default()
                    .global_attachment(&name, "text/plain", b"runner output")
                    .expect("global attachment should be recorded");
            });

            let out_dir = crate::results_dir_from_env();
            let globals = fs::read_dir(&out_dir)
                .expect("results dir should exist")
                .map(|entry| entry.expect("results entry should be readable").path())
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.ends_with("-globals.json"))
                })
                .map(|path| {
                    let json = fs::read_to_string(path).expect("globals file should be readable");
                    serde_json::from_str::<serde_json::Value>(&json)
                        .expect("globals file should be valid JSON")
                })
                .find(|globals| globals["attachments"][0]["name"] == name)
                .expect("globals file should reference this attachment");
            attachment(
                "globals-record.json",
                "application/json",
                serde_json::to_vec_pretty(&globals).expect("globals JSON should serialize"),
            );

            let attachment = &globals["attachments"][0];
            assert_eq!(attachment["contentType"], "text/plain");
            let source = attachment["source"]
                .as_str()
                .expect("global attachment source should be a string");
            assert_eq!(
                fs::read(out_dir.join(source)).expect("global attachment should be readable"),
                b"runner output"
            );
            let id = source
                .strip_suffix("-attachment.log")
                .expect("global attachment source should have the expected suffix");
            let uuid = Uuid::parse_str(id)
                .unwrap_or_else(|error| panic!("{id:?} should be a UUID: {error}"));
            assert_eq!(uuid.get_version(), Some(Version::Random));
        },
    );
}

#[test]
fn enter_step_records_failure_without_panicking() {
    let (allure, out_dir) = make_facade("enter_step_fail");
    allure.start_test_case(StartTestCaseParams::new("enter_step_fail"));
    {
        let mut step = allure.enter_step("failing step");
        step.fail("boom");
    }
    allure.stop_test_case(Status::Passed, None);

    let result = read_result(&out_dir);
    let step = &result["steps"][0];
    assert_eq!(step["name"], "failing step");
    assert_eq!(step["status"], "failed");
    assert_eq!(step["statusDetails"]["message"], "boom");
}

#[test]
fn enter_step_defaults_to_passed() {
    let (allure, out_dir) = make_facade("enter_step_pass");
    allure.start_test_case(StartTestCaseParams::new("enter_step_pass"));
    allure.enter_step("plain step").finish();
    allure.stop_test_case(Status::Passed, None);

    let result = read_result(&out_dir);
    assert_eq!(result["steps"][0]["name"], "plain step");
    assert_eq!(result["steps"][0]["status"], "passed");
}
