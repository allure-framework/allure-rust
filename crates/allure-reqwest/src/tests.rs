use super::*;
use allure_cargotest::{allure_test, CargoTestReporter};
use allure_rust_commons::{
    apply_config_labels, attachment as allure_attachment, description as allure_description,
    step as allure_step, title_path, AllureFacade, HTTP_EXCHANGE_ATTACHMENT_MIME,
};
use serde_json::Value;
use std::{
    fs,
    future::Future,
    io::{Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
    time::Duration,
};

fn make_results_dir(test_name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "allure-reqwest-tests-{test_name}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos()
    ))
}

// The helpers below make each Rust test create and then inspect its own Allure result.
fn run_within_test_context<F>(test_name: &str, body: F) -> (Value, Vec<Value>)
where
    F: FnOnce(AllureFacade),
{
    let out_dir = make_results_dir(test_name);
    let reporter = CargoTestReporter::new(&out_dir).expect("reporter should initialize");
    let full_name = format!("allure_reqwest::tests::{test_name}");

    reporter.run_test_with_metadata(test_name, Some(&full_name), None, None, |allure| {
        apply_test_metadata(allure);
        body(allure.clone());
    });

    read_allure_result(&out_dir, test_name)
}

fn run_async_within_test_context<F, Fut>(test_name: &str, body: F) -> (Value, Vec<Value>)
where
    F: FnOnce(AllureFacade) -> Fut,
    Fut: Future<Output = ()>,
{
    let out_dir = make_results_dir(test_name);
    let reporter = CargoTestReporter::new(&out_dir).expect("reporter should initialize");
    let allure = reporter.allure().clone();
    let full_name = format!("allure_reqwest::tests::{test_name}");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build");

    runtime.block_on(reporter.run_test_with_metadata_async(
        test_name,
        Some(&full_name),
        None,
        None,
        async move {
            apply_test_metadata(&allure);
            body(allure).await;
        },
    ));

    read_allure_result(&out_dir, test_name)
}

fn apply_test_metadata(allure: &AllureFacade) {
    let title_path = title_path(file!(), env!("CARGO_MANIFEST_DIR"));
    apply_config_labels(
        allure,
        env!("CARGO_MANIFEST_DIR"),
        module_path!(),
        &title_path,
    );
    allure.title_path(title_path);
}

fn read_allure_result(out_dir: &Path, test_name: &str) -> (Value, Vec<Value>) {
    let mut result = None;
    for entry in fs::read_dir(out_dir).expect("results dir should exist") {
        let path = entry.expect("result dir entry should be readable").path();
        if !path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.ends_with("-result.json"))
            .unwrap_or(false)
        {
            continue;
        }

        let value = serde_json::from_str::<Value>(
            &fs::read_to_string(path).expect("result json should be readable"),
        )
        .expect("result json should parse");
        if value["name"] == test_name {
            result = Some(value);
            break;
        }
    }

    let result = result.expect("expected test result should exist");
    let mut attachment_sources = Vec::new();
    collect_attachment_sources(&result, &mut attachment_sources);
    let attachments = attachment_sources
        .into_iter()
        .map(|source| {
            serde_json::from_str::<Value>(
                &fs::read_to_string(out_dir.join(source)).expect("attachment should be readable"),
            )
            .expect("attachment json should parse")
        })
        .collect();

    (result, attachments)
}

fn collect_attachment_sources(value: &Value, sources: &mut Vec<String>) {
    if let Some(attachments) = value["attachments"].as_array() {
        for attachment in attachments {
            let source = attachment["source"]
                .as_str()
                .expect("attachment source should be a string");
            sources.push(source.to_string());
        }
    }

    if let Some(steps) = value["steps"].as_array() {
        for step in steps {
            collect_attachment_sources(step, sources);
        }
    }
}

fn assert_reported_to_allure(result: &Value, test_name: &str) {
    assert_eq!(result["name"], test_name);
    assert_eq!(result["status"], "passed");
    assert!(contains_label(result, "module", env!("CARGO_PKG_NAME")));
    assert!(contains_label(result, "language", "rust"));
    assert!(contains_label(result, "framework", "cargo-test"));
    assert_eq!(result["titlePath"], serde_json::json!(["src", "tests.rs"]));
}

fn assert_wrapped_attachment(result: &Value, name: &str) {
    assert!(result["attachments"]
        .as_array()
        .expect("root attachments should be an array")
        .is_empty());
    assert_eq!(result["steps"][0]["name"], name);
    assert_eq!(result["steps"][0]["status"], "passed");
    assert_eq!(result["steps"][0]["attachments"][0]["name"], name);
    assert!(result["steps"][0]["attachments"][0]["source"]
        .as_str()
        .expect("attachment source should be a string")
        .ends_with(".httpexchange"));
}

fn attach_json_evidence(name: &str, content_type: &str, value: &Value) {
    let body = serde_json::to_vec_pretty(value).expect("evidence json should serialize");
    allure_attachment(name, content_type, body);
}

fn attach_http_exchange_evidence(name: &str, exchange: &Value) {
    attach_json_evidence(name, HTTP_EXCHANGE_ATTACHMENT_MIME, exchange);
}

fn attach_received_request_evidence(request: &ReceivedRequest) {
    let headers = request
        .headers
        .iter()
        .map(|(name, value)| {
            let value = if name.eq_ignore_ascii_case("authorization") {
                HTTP_EXCHANGE_REDACTED_VALUE
            } else {
                value
            };
            serde_json::json!({
                "name": name,
                "value": value,
            })
        })
        .collect::<Vec<_>>();
    let payload = serde_json::json!({
        "method": request.method.as_str(),
        "path": request.path.replace("token=secret", "token=__ALLURE_REDACTED__"),
        "headers": headers,
        "body": request.body.as_str(),
    });

    attach_json_evidence(
        "request observed by local server",
        "application/json",
        &payload,
    );
}

fn contains_label(result: &Value, name: &str, value: &str) -> bool {
    result["labels"]
        .as_array()
        .expect("labels should be an array")
        .iter()
        .any(|label| label["name"] == name && label["value"] == value)
}

#[derive(Debug)]
struct ReceivedRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: String,
}

impl ReceivedRequest {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header_name, _)| header_name.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

struct TestServer {
    url: String,
    received: Receiver<ReceivedRequest>,
    thread: JoinHandle<()>,
}

impl TestServer {
    fn spawn(status: &str, headers: &[(&str, &str)], body: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
        let addr = listener
            .local_addr()
            .expect("test server address should be available");
        let status = status.to_string();
        let headers = headers
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect::<Vec<_>>();
        let body = body.to_string();
        let (sender, received) = mpsc::channel();

        let thread = thread::spawn(move || {
            let (mut stream, _) = listener
                .accept()
                .expect("test server should accept one request");
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("test server should set read timeout");
            let request = read_request(&mut stream);
            sender
                .send(request)
                .expect("received request should be delivered");

            let mut response = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\n", body.len());
            for (name, value) in headers {
                response.push_str(&format!("{name}: {value}\r\n"));
            }
            response.push_str("\r\n");
            response.push_str(&body);
            stream
                .write_all(response.as_bytes())
                .expect("test response should write");
        });

        Self {
            url: format!("http://{addr}"),
            received,
            thread,
        }
    }

    fn url(&self) -> &str {
        &self.url
    }

    fn received_request(self) -> ReceivedRequest {
        let request = self
            .received
            .recv_timeout(Duration::from_secs(5))
            .expect("test server should receive one request");
        self.thread.join().expect("test server should stop cleanly");
        request
    }
}

fn read_request(stream: &mut std::net::TcpStream) -> ReceivedRequest {
    let mut bytes = Vec::new();
    let header_end = loop {
        let mut buffer = [0_u8; 1024];
        let read = stream
            .read(&mut buffer)
            .expect("test server should read request");
        assert!(read > 0, "request should not close before headers");
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(index) = find_header_end(&bytes) {
            break index;
        }
    };

    let header_bytes = &bytes[..header_end];
    let header_text = std::str::from_utf8(header_bytes).expect("request headers should be utf-8");
    let mut lines = header_text.split("\r\n");
    let request_line = lines.next().expect("request line should exist");
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .expect("request method should exist")
        .to_string();
    let path = request_parts
        .next()
        .expect("request path should exist")
        .to_string();

    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.to_string(), value.trim().to_string()))
        .collect::<Vec<_>>();
    let content_length = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse::<usize>().ok())
        .unwrap_or(0);

    let body_start = header_end + 4;
    while bytes.len().saturating_sub(body_start) < content_length {
        let mut buffer = [0_u8; 1024];
        let read = stream
            .read(&mut buffer)
            .expect("test server should read request body");
        assert!(read > 0, "request should not close before body");
        bytes.extend_from_slice(&buffer[..read]);
    }

    let body_bytes = &bytes[body_start..body_start + content_length];
    let body = String::from_utf8(body_bytes.to_vec()).expect("request body should be utf-8");

    ReceivedRequest {
        method,
        path,
        headers,
        body,
    }
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

#[allure_test]
#[test]
fn captures_request_response_metadata_and_request_body() {
    allure_description(
        "Verifies that the reqwest client captures request metadata, redacts sensitive query/header values, records the request body, and preserves response status metadata without response body capture.",
    );
    let server = allure_step("start local server returning 201 JSON response", || {
        TestServer::spawn(
            "201 Created",
            &[("content-type", "application/json"), ("x-trace", "abc")],
            r#"{"id":42}"#,
        )
    });
    let url = server.url().to_string();
    let (result, attachments) = allure_step(
        "send redacted POST through AllureReqwestClient",
        || {
            run_async_within_test_context(
                "captures_request_response_metadata_and_request_body",
                |allure| async move {
                    allure.description(
                        "Captures a POST request with redacted token and authorization values plus request body and response status metadata.",
                    );
                    let client = AllureReqwestClient::new(allure);

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

                    assert_eq!(response.status(), 201);
                },
            )
        },
    );
    let received = allure_step("collect request observed by local server", || {
        server.received_request()
    });

    allure_step(
        "verify request metadata and redacted HTTP exchange evidence",
        || {
            assert_reported_to_allure(
                &result,
                "captures_request_response_metadata_and_request_body",
            );
            attach_received_request_evidence(&received);
            assert_eq!(received.method, "POST");
            assert_eq!(received.path, "/v1/orders?dryRun=true&token=secret");
            assert_eq!(received.header("authorization"), Some("Bearer secret"));
            assert_eq!(received.header("content-type"), Some("application/json"));
            assert_eq!(received.body, r#"{"name":"demo"}"#);
            assert_wrapped_attachment(&result, "HTTP Exchange");
            let attachment = attachments
                .first()
                .expect("http exchange attachment should exist");
            attach_http_exchange_evidence("captured redacted HTTP exchange", attachment);
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
        },
    );
}

#[allure_test]
#[test]
fn captures_response_body_when_enabled_and_preserves_body_for_caller() {
    allure_description(
        "Verifies that opt-in response body capture records JSON response content while leaving the caller able to read the body.",
    );
    let server = allure_step("start local server returning 200 JSON response", || {
        TestServer::spawn(
            "200 OK",
            &[("content-type", "application/json")],
            r#"{"ok":true}"#,
        )
    });
    let url = server.url().to_string();
    let (result, attachments) = allure_step("send GET with response body capture enabled", || {
        run_async_within_test_context(
            "captures_response_body_when_enabled_and_preserves_body_for_caller",
            |allure| async move {
                allure.description(
                        "Captures the response body in the HTTP exchange and still returns a readable reqwest response body to the caller.",
                    );
                let client = AllureReqwestClient::new(allure).with_options(
                    CaptureOptions::default()
                        .with_attachment_name("Create order")
                        .with_response_body_capture(1024),
                );

                let response = client
                    .send(client.get(format!("{url}/v1/orders/42")))
                    .await
                    .expect("request should succeed");
                let body = response
                    .text()
                    .await
                    .expect("response body should be readable");

                assert_eq!(body, r#"{"ok":true}"#);
            },
        )
    });
    let received = allure_step("collect request observed by local server", || {
        server.received_request()
    });

    allure_step(
        "verify response body capture and caller-visible body",
        || {
            assert_reported_to_allure(
                &result,
                "captures_response_body_when_enabled_and_preserves_body_for_caller",
            );
            attach_received_request_evidence(&received);
            assert_eq!(received.method, "GET");
            assert_eq!(received.path, "/v1/orders/42");
            assert_eq!(received.body, "");
            assert_wrapped_attachment(&result, "Create order");
            let attachment = attachments
                .first()
                .expect("http exchange attachment should exist");
            attach_http_exchange_evidence("captured response body exchange", attachment);
            assert_eq!(
                attachment["response"]["body"]["contentType"],
                "application/json"
            );
            assert_eq!(attachment["response"]["body"]["encoding"], "utf8");
            assert_eq!(attachment["response"]["body"]["value"], r#"{"ok":true}"#);
        },
    );
}

#[allure_test]
#[test]
fn body_capture_truncates_and_base64_encodes_binary() {
    allure_description(
        "Verifies that binary body capture uses base64, records original size, and marks payloads truncated when the configured byte limit is exceeded.",
    );
    let (result, attachments) = allure_step(
        "encode binary body with a three-byte capture limit",
        || {
            run_within_test_context(
                "body_capture_truncates_and_base64_encodes_binary",
                |allure| {
                    allure.description(
                "Encodes a binary body as base64 and marks it truncated when only the first three bytes are captured.",
            );
                    let body =
                        body_from_bytes(&[0, 159, 146, 150, 255], Some("image/png".to_string()), 3);

                    assert_eq!(body.content_type.as_deref(), Some("image/png"));
                    assert!(matches!(
                        body.encoding,
                        Some(HttpExchangeBodyEncoding::Base64)
                    ));
                    assert_eq!(body.value.as_deref(), Some("AJ+S"));
                    assert_eq!(body.size, Some(5));
                    assert_eq!(body.truncated, Some(true));
                },
            )
        },
    );

    allure_step(
        "verify encoded binary body metadata and absence of HTTP attachments",
        || {
            assert_reported_to_allure(&result, "body_capture_truncates_and_base64_encodes_binary");
            assert!(attachments.is_empty());
        },
    );
}

#[cfg(feature = "middleware")]
#[allure_test]
#[test]
fn middleware_can_be_constructed() {
    allure_description(
        "Verifies that the optional reqwest-middleware integration can be constructed with custom capture options.",
    );
    let (result, attachments) = allure_step(
        "construct middleware with custom attachment name",
        || {
            run_within_test_context("middleware_can_be_constructed", |allure| {
                allure.description(
                "Constructs the middleware adapter and applies a custom HTTP exchange attachment name.",
            );
                let _middleware = AllureReqwestMiddleware::new(allure)
                    .with_options(CaptureOptions::default().with_attachment_name("HTTP"));
            })
        },
    );

    allure_step("verify middleware construction result metadata", || {
        assert_reported_to_allure(&result, "middleware_can_be_constructed");
        assert!(attachments.is_empty());
    });
}
