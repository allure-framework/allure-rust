use super::*;
use crate::model::{
    Attachment, Categories, Category, GlobalAttachment, GlobalError, Globals, Status, TestResult,
    TestResultContainer,
};
use std::time::{SystemTime, UNIX_EPOCH};

fn make_writer() -> FileSystemResultsWriter {
    let dir = std::env::temp_dir().join(format!(
        "allure-rust-writer-tests-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos()
    ));
    FileSystemResultsWriter::new(dir).expect("writer should initialize")
}

fn read_to_string(path: &Path) -> String {
    fs::read_to_string(path).expect("written file should be readable")
}

#[test]
fn write_result_wrapper_writes_result_json() {
    let writer = make_writer();
    let result = TestResult {
        uuid: "result-1".to_string(),
        name: "line1\n\"quoted\"\\slash".to_string(),
        ..Default::default()
    };

    let path = writer.write_result(&result).expect("result should write");

    assert!(path.ends_with("result-1-result.json"));
    assert!(read_to_string(&path).contains("line1\\n\\\"quoted\\\"\\\\slash"));
}

#[test]
fn write_result_typed_writes_result_json() {
    let writer = make_writer();
    let result = TestResult {
        uuid: "result-2".to_string(),
        name: "typed result".to_string(),
        status: Some(Status::Passed),
        ..Default::default()
    };

    let path = writer
        .write_result_typed(&result)
        .expect("typed result should write");

    let json = read_to_string(&path);
    assert!(path.ends_with("result-2-result.json"));
    assert!(json.contains("\"status\":\"passed\""));
}

#[test]
fn write_container_and_typed_write_container_json() {
    let writer = make_writer();
    let container = TestResultContainer {
        uuid: "container-1".to_string(),
        children: vec!["child".to_string()],
        ..Default::default()
    };

    let wrapper_path = writer
        .write_container(&container)
        .expect("container wrapper should write");
    let typed_path = writer
        .write_container_typed(&TestResultContainer {
            uuid: "container-2".to_string(),
            ..Default::default()
        })
        .expect("container typed should write");

    assert!(wrapper_path.ends_with("container-1-container.json"));
    assert!(typed_path.ends_with("container-2-container.json"));
}

#[test]
fn write_globals_and_typed_write_globals_json() {
    let writer = make_writer();
    let globals = Globals {
        attachments: vec![GlobalAttachment {
            name: "global-att".to_string(),
            source: "source.bin".to_string(),
            content_type: "application/octet-stream".to_string(),
        }],
        errors: vec![GlobalError {
            message: "boom".to_string(),
            trace: None,
        }],
    };

    let path_1 = writer
        .write_globals(&globals)
        .expect("globals wrapper should write");
    let path_2 = writer
        .write_globals_typed(&globals)
        .expect("globals typed should write");

    let file_name_1 = path_1
        .file_name()
        .and_then(|v| v.to_str())
        .expect("globals wrapper file name should be valid utf-8");
    let file_name_2 = path_2
        .file_name()
        .and_then(|v| v.to_str())
        .expect("globals typed file name should be valid utf-8");

    assert!(file_name_1.ends_with("-globals.json"));
    assert!(file_name_2.ends_with("-globals.json"));
    assert!(read_to_string(&path_1).contains("global-att"));
    assert!(read_to_string(&path_2).contains("\"errors\""));
}

#[test]
fn write_environment_properties_writes_sorted_pairs() {
    let writer = make_writer();
    let properties = HashMap::from([
        ("z_key".to_string(), "z".to_string()),
        ("a_key".to_string(), "a".to_string()),
    ]);

    let path = writer
        .write_environment_properties(&properties)
        .expect("environment properties should write");

    assert!(path.ends_with("environment.properties"));
    assert_eq!(read_to_string(&path), "a_key=a\nz_key=z");
}

#[test]
fn write_categories_and_typed_write_categories_json() {
    let writer = make_writer();
    let categories = Categories(vec![Category {
        name: "cat".to_string(),
        description: Some("desc".to_string()),
        matched_statuses: Some(vec![Status::Failed]),
        message_regex: Some("error.*".to_string()),
        trace_regex: None,
        flaky: Some(false),
    }]);

    let path_1 = writer
        .write_categories(&categories)
        .expect("categories wrapper should write");
    let path_2 = writer
        .write_categories_typed(&categories)
        .expect("categories typed should write");

    assert!(path_1.ends_with("categories.json"));
    assert!(path_2.ends_with("categories.json"));
    assert!(read_to_string(&path_1).contains("error.*"));
}

#[test]
fn write_attachment_methods_write_expected_files() {
    let writer = make_writer();

    let path_1 = writer
        .write_attachment("manual-1.bin", b"first")
        .expect("attachment wrapper should write");
    let path_2 = writer
        .write_attachment_named("manual-2.bin", b"second")
        .expect("attachment named should write");
    let (source, path_3) = writer
        .write_attachment_auto("att-uuid", Some("report.json"), Some("text/plain"), b"auto")
        .expect("attachment auto should write");

    assert!(path_1.ends_with("manual-1.bin"));
    assert_eq!(
        fs::read(&path_1).expect("first attachment should exist"),
        b"first"
    );
    assert!(path_2.ends_with("manual-2.bin"));
    assert_eq!(
        fs::read(&path_2).expect("second attachment should exist"),
        b"second"
    );
    assert_eq!(source, "att-uuid-attachment.json");
    assert!(path_3.ends_with("att-uuid-attachment.json"));
    assert_eq!(
        fs::read(&path_3).expect("auto attachment should exist"),
        b"auto"
    );
}

#[test]
fn serializes_with_escaped_strings() {
    let result = TestResult {
        uuid: "u-1".to_string(),
        name: "line1\n\"quoted\"\\slash".to_string(),
        status: Some(Status::Passed),
        attachments: vec![Attachment {
            name: "att\n\"n\"".to_string(),
            source: "s\\x".to_string(),
            content_type: "application/json".to_string(),
        }],
        ..Default::default()
    };

    let json = serde_json::to_string(&result).expect("test result should serialize");

    assert!(json.contains("line1\\n\\\"quoted\\\"\\\\slash"));
    assert!(json.contains("att\\n\\\"n\\\""));
    assert!(json.contains("s\\\\x"));
}

#[test]
fn attachment_filename_prefers_name_extension() {
    let source = attachment_source_name("abc", Some("report.custom"), Some("application/json"));
    assert_eq!(source, "abc-attachment.custom");
}

#[test]
fn attachment_filename_falls_back_to_content_type() {
    let source = attachment_source_name(
        "abc",
        Some("report"),
        Some("application/json; charset=utf-8"),
    );
    assert_eq!(source, "abc-attachment.json");
}

#[test]
fn attachment_filename_has_no_extension_when_unknown() {
    let source = attachment_source_name("abc", Some("report"), Some("application/x-custom"));
    assert_eq!(source, "abc-attachment");
}
