use super::*;
use allure_rust_commons::{AllureRuntime, FileSystemResultsWriter, Status};
use serde_json::Value;
use std::{fs, path::PathBuf};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

fn make_allure(test_name: &str) -> (AllureFacade, allure_rust_commons::AllureLifecycle, PathBuf) {
    let out_dir = std::env::temp_dir().join(format!(
        "allure-reqwest-tests-{test_name}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos()
    ));
    let runtime = AllureRuntime::new(
        FileSystemResultsWriter::new(&out_dir).expect("writer should initialize"),
    );
    let lifecycle = runtime.lifecycle();
    let allure = AllureFacade::with_lifecycle(lifecycle.clone());
    (allure, lifecycle, out_dir)
}

async fn spawn_server(status: &str, headers: &[(&str, &str)], body: &str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test server should bind");
    let addr = listener
        .local_addr()
        .expect("test server address should be available");
    let status = status.to_string();
    let headers = headers
        .iter()
        .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
        .collect::<Vec<_>>();
    let body = body.to_string();

    tokio::spawn(async move {
        let (mut socket, _) = listener
            .accept()
            .await
            .expect("test server should accept one request");
        let mut buffer = [0_u8; 4096];
        let _ = socket.read(&mut buffer).await;

        let mut response = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\n", body.len());
        for (name, value) in headers {
            response.push_str(&format!("{name}: {value}\r\n"));
        }
        response.push_str("\r\n");
        response.push_str(&body);
        socket
            .write_all(response.as_bytes())
            .await
            .expect("test response should write");
    });

    format!("http://{addr}")
}

fn read_single_attachment(out_dir: &PathBuf) -> (Value, Value) {
    let result_path = fs::read_dir(out_dir)
        .expect("result dir should exist")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.ends_with("-result.json"))
                .unwrap_or(false)
        })
        .expect("result json should exist");
    let result = serde_json::from_str::<Value>(
        &fs::read_to_string(result_path).expect("result json should be readable"),
    )
    .expect("result json should parse");
    let source = result["attachments"][0]["source"]
        .as_str()
        .expect("attachment source should be a string");
    let attachment = serde_json::from_str::<Value>(
        &fs::read_to_string(out_dir.join(source)).expect("attachment should be readable"),
    )
    .expect("attachment json should parse");
    (result, attachment)
}

#[tokio::test]
async fn captures_request_response_metadata_and_request_body() {
    let (allure, lifecycle, out_dir) = make_allure("metadata");
    let url = spawn_server(
        "201 Created",
        &[("content-type", "application/json"), ("x-trace", "abc")],
        r#"{"id":42}"#,
    )
    .await;
    let client = AllureReqwestClient::new(allure);

    lifecycle.start_test_case("metadata");
    let response = client
        .send(
            client
                .post(format!("{url}/v1/orders?dryRun=true&token=secret"))
                .header("authorization", "Bearer secret")
                .header("content-type", "application/json")
                .body(r#"{"name":"demo"}"#),
        )
        .await
        .expect("request should succeed");
    lifecycle.stop_test_case(Status::Passed, None);

    assert_eq!(response.status(), 201);
    let (result, attachment) = read_single_attachment(&out_dir);
    assert_eq!(result["attachments"][0]["name"], "HTTP Exchange");
    assert!(result["attachments"][0]["source"]
        .as_str()
        .expect("attachment source should be a string")
        .ends_with(".httpexchange"));
    assert_eq!(attachment["schemaVersion"], 1);
    assert_eq!(attachment["request"]["method"], "POST");
    assert_eq!(attachment["request"]["query"][0]["name"], "dryRun");
    assert_eq!(attachment["request"]["query"][0]["value"], "true");
    assert_eq!(attachment["request"]["query"][1]["name"], "token");
    assert_eq!(
        attachment["request"]["query"][1]["value"],
        HTTP_EXCHANGE_REDACTED_VALUE
    );
    assert_eq!(
        attachment["request"]["headers"][0]["value"],
        HTTP_EXCHANGE_REDACTED_VALUE
    );
    assert_eq!(attachment["request"]["body"]["encoding"], "utf8");
    assert_eq!(attachment["request"]["body"]["value"], r#"{"name":"demo"}"#);
    assert_eq!(attachment["response"]["status"], 201);
    assert_eq!(attachment["response"]["statusText"], "Created");
    assert!(attachment["response"].get("body").is_none());
}

#[tokio::test]
async fn captures_response_body_when_enabled_and_preserves_body_for_caller() {
    let (allure, lifecycle, out_dir) = make_allure("response-body");
    let url = spawn_server(
        "200 OK",
        &[("content-type", "application/json")],
        r#"{"ok":true}"#,
    )
    .await;
    let client = AllureReqwestClient::new(allure).with_options(
        CaptureOptions::default()
            .with_attachment_name("Create order")
            .with_response_body_capture(1024),
    );

    lifecycle.start_test_case("response-body");
    let response = client
        .send(client.get(format!("{url}/v1/orders/42")))
        .await
        .expect("request should succeed");
    let body = response
        .text()
        .await
        .expect("response body should be readable");
    lifecycle.stop_test_case(Status::Passed, None);

    assert_eq!(body, r#"{"ok":true}"#);
    let (result, attachment) = read_single_attachment(&out_dir);
    assert_eq!(result["attachments"][0]["name"], "Create order");
    assert_eq!(
        attachment["response"]["body"]["contentType"],
        "application/json"
    );
    assert_eq!(attachment["response"]["body"]["encoding"], "utf8");
    assert_eq!(attachment["response"]["body"]["value"], r#"{"ok":true}"#);
}

#[test]
fn body_capture_truncates_and_base64_encodes_binary() {
    let body = body_from_bytes(&[0, 159, 146, 150, 255], Some("image/png".to_string()), 3);

    assert_eq!(body.content_type.as_deref(), Some("image/png"));
    assert!(matches!(
        body.encoding,
        Some(HttpExchangeBodyEncoding::Base64)
    ));
    assert_eq!(body.value.as_deref(), Some("AJ+S"));
    assert_eq!(body.size, Some(5));
    assert_eq!(body.truncated, Some(true));
}

#[cfg(feature = "middleware")]
#[test]
fn middleware_can_be_constructed() {
    let (allure, _, _) = make_allure("middleware-construct");
    let _middleware = AllureReqwestMiddleware::new(allure)
        .with_options(CaptureOptions::default().with_attachment_name("HTTP"));
}
