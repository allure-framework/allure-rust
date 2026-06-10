use std::{
    panic::Location,
    panic::{catch_unwind, resume_unwind, AssertUnwindSafe},
    time::{SystemTime, UNIX_EPOCH},
};

use allure_rust_commons::{
    md5_hex, title_path, FileSystemResultsWriter, Label, Stage, Status, StatusDetails, StepResult,
    TestResult,
};

#[track_caller]
pub(crate) fn allure_test<F>(module_path: &str, test_name: &str, body: F)
where
    F: FnOnce(),
{
    let writer = FileSystemResultsWriter::from_env().expect("allure writer should be created");
    let full_name = format!("{module_path}::{test_name}");
    let started_at = now_millis();

    let result = catch_unwind(AssertUnwindSafe(body));
    let stopped_at = now_millis();
    let history_id = md5_hex(&full_name);
    let uuid = format!("cargo-test-{history_id}");
    let title_path = title_path(Location::caller().file(), env!("CARGO_MANIFEST_DIR"));

    match result {
        Ok(()) => {
            write_result(
                &writer,
                TestResult {
                    uuid,
                    name: test_name.to_string(),
                    full_name: Some(full_name),
                    history_id: Some(history_id.clone()),
                    test_case_id: Some(history_id),
                    status: Some(Status::Passed),
                    labels: default_labels(),
                    steps: vec![body_step(Status::Passed, None, started_at, stopped_at)],
                    title_path: Some(title_path),
                    start: Some(started_at),
                    stop: Some(stopped_at),
                    ..Default::default()
                },
            );
        }
        Err(payload) => {
            let message = if let Some(message) = payload.downcast_ref::<&str>() {
                (*message).to_string()
            } else if let Some(message) = payload.downcast_ref::<String>() {
                message.clone()
            } else {
                "panic without string payload".to_string()
            };
            let details = StatusDetails {
                message: Some(message),
                trace: None,
                actual: None,
                expected: None,
            };
            write_result(
                &writer,
                TestResult {
                    uuid,
                    name: test_name.to_string(),
                    full_name: Some(full_name),
                    history_id: Some(history_id.clone()),
                    test_case_id: Some(history_id),
                    status: Some(Status::Failed),
                    status_details: Some(details.clone()),
                    labels: default_labels(),
                    steps: vec![body_step(
                        Status::Failed,
                        Some(details),
                        started_at,
                        stopped_at,
                    )],
                    title_path: Some(title_path),
                    start: Some(started_at),
                    stop: Some(stopped_at),
                    ..Default::default()
                },
            );
            resume_unwind(payload);
        }
    }
}

fn body_step(
    status: Status,
    status_details: Option<StatusDetails>,
    start: i64,
    stop: i64,
) -> StepResult {
    StepResult {
        name: "execute test body".to_string(),
        status: Some(status),
        status_details,
        stage: Some(Stage::Finished),
        start: Some(start),
        stop: Some(stop),
        ..Default::default()
    }
}

fn write_result(writer: &FileSystemResultsWriter, result: TestResult) {
    writer
        .write_result_typed(&result)
        .expect("allure test result should be written");
}

fn default_labels() -> Vec<Label> {
    vec![
        Label {
            name: "language".to_string(),
            value: "rust".to_string(),
        },
        Label {
            name: "framework".to_string(),
            value: "cargo-test".to_string(),
        },
        Label {
            name: "module".to_string(),
            value: env!("CARGO_PKG_NAME").to_string(),
        },
    ]
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
