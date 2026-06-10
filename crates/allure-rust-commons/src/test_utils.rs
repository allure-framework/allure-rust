use std::{
    panic::Location,
    panic::{catch_unwind, resume_unwind, AssertUnwindSafe},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{error_classifier, md5_hex, FileSystemResultsWriter, Label, Status, TestResult};

#[track_caller]
pub(crate) fn allure_test<F>(module_path: &str, test_name: &str, body: F)
where
    F: FnOnce(),
{
    let results_dir =
        std::env::var("ALLURE_RESULTS_DIR").unwrap_or_else(|_| "target/allure-results".to_string());
    let writer =
        FileSystemResultsWriter::new(results_dir).expect("allure writer should be created");
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
                    title_path: Some(title_path),
                    start: Some(started_at),
                    stop: Some(stopped_at),
                    ..Default::default()
                },
            );
        }
        Err(payload) => {
            let (status, details) = error_classifier::classify_panic(&payload);
            write_result(
                &writer,
                TestResult {
                    uuid,
                    name: test_name.to_string(),
                    full_name: Some(full_name),
                    history_id: Some(history_id.clone()),
                    test_case_id: Some(history_id),
                    status: Some(status),
                    status_details: Some(details),
                    labels: default_labels(),
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

fn title_path(file: &str, manifest_dir: &str) -> Vec<String> {
    relative_file_path(file, manifest_dir)
        .split('/')
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn relative_file_path(file: &str, manifest_dir: &str) -> String {
    let file = file.replace('\\', "/");
    let manifest_dir = manifest_dir.replace('\\', "/");
    if let Some(relative) = file
        .strip_prefix(&manifest_dir)
        .map(|path| path.trim_start_matches('/'))
    {
        return relative.to_string();
    }

    let Some(package_name) = manifest_dir.rsplit('/').next() else {
        return file;
    };
    let package_segment = format!("/{package_name}/");
    if let Some((_, relative)) = file.split_once(&package_segment) {
        return relative.to_string();
    }
    let package_prefix = format!("{package_name}/");
    if let Some(relative) = file.strip_prefix(&package_prefix) {
        return relative.to_string();
    }

    file
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
