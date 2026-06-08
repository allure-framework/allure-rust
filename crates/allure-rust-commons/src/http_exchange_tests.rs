use super::*;
use crate::test_utils::allure_test;
use serde_json::json;

#[test]
fn serializes_minimal_payload_and_omits_missing_optional_fields() {
    allure_test(
        module_path!(),
        "serializes_minimal_payload_and_omits_missing_optional_fields",
        || {
            let exchange = HttpExchange::new("GET", "https://api.example.com/v1/orders/42");

            let value = serde_json::to_value(&exchange).expect("http exchange should serialize");

            assert_eq!(
                value,
                json!({
                    "schemaVersion": 1,
                    "request": {
                        "method": "GET",
                        "url": "https://api.example.com/v1/orders/42"
                    }
                })
            );
        },
    );
}

#[test]
fn serializes_typical_payload() {
    allure_test(module_path!(), "serializes_typical_payload", || {
        let mut exchange =
            HttpExchange::new("POST", "https://api.example.com/v1/orders/42?dryRun=true");
        exchange.start = Some(1_710_000_186_400);
        exchange.stop = Some(1_710_000_186_487);
        exchange.request.http_version = Some("HTTP/1.1".to_string());
        exchange.request.cookies = Some(vec![HttpExchangeCookie {
            name: "sid".to_string(),
            value: HTTP_EXCHANGE_REDACTED_VALUE.to_string(),
            path: Some("/".to_string()),
            http_only: Some(true),
            secure: Some(true),
            ..Default::default()
        }]);
        exchange.request.headers = Some(vec![
            HttpExchangeNameValue::new("Authorization", HTTP_EXCHANGE_REDACTED_VALUE),
            HttpExchangeNameValue::new("content-type", "application/json"),
        ]);
        exchange.request.query = Some(vec![HttpExchangeNameValue::new("dryRun", "true")]);
        exchange.request.body = Some(HttpExchangeBody {
            content_type: Some("application/json".to_string()),
            encoding: Some(HttpExchangeBodyEncoding::Utf8),
            value: Some("{\"name\":\"demo\",\"quantity\":1}".to_string()),
            size: Some(28),
            truncated: Some(false),
            ..Default::default()
        });
        exchange.response = Some(HttpExchangeResponse {
            status: Some(201),
            status_text: Some("Created".to_string()),
            http_version: Some("HTTP/1.1".to_string()),
            headers: Some(vec![HttpExchangeNameValue::new(
                "content-type",
                "application/json",
            )]),
            body: Some(HttpExchangeBody {
                content_type: Some("application/json".to_string()),
                encoding: Some(HttpExchangeBodyEncoding::Utf8),
                value: Some("{\"id\":42}".to_string()),
                size: Some(9),
                truncated: Some(false),
                ..Default::default()
            }),
            ..Default::default()
        });

        let value = serde_json::to_value(&exchange).expect("http exchange should serialize");

        assert_eq!(
            value,
            json!({
                "schemaVersion": 1,
                "start": 1710000186400i64,
                "stop": 1710000186487i64,
                "request": {
                    "method": "POST",
                    "url": "https://api.example.com/v1/orders/42?dryRun=true",
                    "httpVersion": "HTTP/1.1",
                    "cookies": [
                        {
                            "name": "sid",
                            "value": "__ALLURE_REDACTED__",
                            "httpOnly": true,
                            "path": "/",
                            "secure": true
                        }
                    ],
                    "headers": [
                        {
                            "name": "Authorization",
                            "value": "__ALLURE_REDACTED__"
                        },
                        {
                            "name": "content-type",
                            "value": "application/json"
                        }
                    ],
                    "query": [
                        {
                            "name": "dryRun",
                            "value": "true"
                        }
                    ],
                    "body": {
                        "contentType": "application/json",
                        "encoding": "utf8",
                        "value": "{\"name\":\"demo\",\"quantity\":1}",
                        "size": 28,
                        "truncated": false
                    }
                },
                "response": {
                    "status": 201,
                    "statusText": "Created",
                    "httpVersion": "HTTP/1.1",
                    "headers": [
                        {
                            "name": "content-type",
                            "value": "application/json"
                        }
                    ],
                    "body": {
                        "contentType": "application/json",
                        "encoding": "utf8",
                        "value": "{\"id\":42}",
                        "size": 9,
                        "truncated": false
                    }
                }
            })
        );
    });
}

#[test]
fn serializes_body_encoding_values() {
    allure_test(module_path!(), "serializes_body_encoding_values", || {
        let utf8 = HttpExchangeBody {
            encoding: Some(HttpExchangeBodyEncoding::Utf8),
            ..Default::default()
        };
        let base64 = HttpExchangeBody {
            encoding: Some(HttpExchangeBodyEncoding::Base64),
            ..Default::default()
        };
        let custom = HttpExchangeBody {
            encoding: Some(HttpExchangeBodyEncoding::Other("gzip+base64".to_string())),
            ..Default::default()
        };

        assert_eq!(
            serde_json::to_value(&utf8).expect("utf8 encoding should serialize"),
            json!({ "encoding": "utf8" })
        );
        assert_eq!(
            serde_json::to_value(&base64).expect("base64 encoding should serialize"),
            json!({ "encoding": "base64" })
        );
        assert_eq!(
            serde_json::to_value(&custom).expect("custom encoding should serialize"),
            json!({ "encoding": "gzip+base64" })
        );
    });
}

#[test]
fn serializes_stream_type_field() {
    allure_test(module_path!(), "serializes_stream_type_field", || {
        let stream = HttpExchangeStream {
            stream_type: Some("server-sent-events".to_string()),
            complete: Some(false),
            chunk_count: Some(1),
        };

        assert_eq!(
            serde_json::to_value(&stream).expect("stream should serialize"),
            json!({
                "type": "server-sent-events",
                "complete": false,
                "chunkCount": 1
            })
        );
    });
}
