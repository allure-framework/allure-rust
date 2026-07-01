use super::*;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

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
