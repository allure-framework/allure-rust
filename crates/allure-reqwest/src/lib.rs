//! Reqwest integration for Allure HTTP Exchange attachments.
//!
//! The recommended integration style for broad request coverage is `AllureReqwestMiddleware`,
//! available with the `middleware` feature. Middleware runs around each request made through
//! `reqwest_middleware`, so existing call sites can keep using a reqwest-style client while HTTP
//! exchanges are captured automatically.
//!
//! [`AllureReqwestClient`] is the smaller alternative when you do not want the middleware
//! dependency or only need to instrument selected calls.
//!
//! By default this crate captures method, URL, query, headers, status, timing, and transport
//! errors. In-memory request bodies are captured with a bounded limit. Response body capture is
//! opt-in through [`CaptureOptions`].

#![deny(missing_docs)]

use std::time::{SystemTime, UNIX_EPOCH};

use allure_rust_commons::{
    AllureFacade, HttpExchange, HttpExchangeBody, HttpExchangeBodyEncoding, HttpExchangeError,
    HttpExchangeNameValue, HttpExchangeRequest, HttpExchangeResponse, HTTP_EXCHANGE_REDACTED_VALUE,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::{
    header::{HeaderMap, CONTENT_TYPE},
    Method, Request, RequestBuilder, Response, Url, Version,
};

const DEFAULT_MAX_BODY_SIZE: usize = 64 * 1024;

/// Controls what the reqwest integration stores in HTTP Exchange attachments.
#[derive(Debug, Clone)]
pub struct CaptureOptions {
    attachment_name: Option<String>,
    capture_request_body: bool,
    capture_response_body: bool,
    max_body_size: usize,
    redacted_headers: Vec<String>,
    redacted_query_params: Vec<String>,
}

impl CaptureOptions {
    /// Sets the attachment step/name used for captured HTTP exchanges.
    pub fn with_attachment_name(mut self, attachment_name: impl Into<String>) -> Self {
        self.attachment_name = Some(attachment_name.into());
        self
    }

    /// Disables request body capture.
    pub fn without_request_body_capture(mut self) -> Self {
        self.capture_request_body = false;
        self
    }

    /// Enables response body capture with the given maximum body size.
    pub fn with_response_body_capture(mut self, max_body_size: usize) -> Self {
        self.capture_response_body = true;
        self.max_body_size = max_body_size;
        self
    }

    /// Disables response body capture.
    pub fn without_response_body_capture(mut self) -> Self {
        self.capture_response_body = false;
        self
    }

    /// Sets the maximum number of body bytes to capture.
    pub fn with_max_body_size(mut self, max_body_size: usize) -> Self {
        self.max_body_size = max_body_size;
        self
    }

    /// Redacts a header by name.
    pub fn redact_header(mut self, name: impl Into<String>) -> Self {
        self.redacted_headers.push(normalize_name(name.into()));
        self
    }

    /// Redacts a query parameter by name.
    pub fn redact_query_param(mut self, name: impl Into<String>) -> Self {
        self.redacted_query_params.push(normalize_name(name.into()));
        self
    }
}

impl Default for CaptureOptions {
    fn default() -> Self {
        Self {
            attachment_name: None,
            capture_request_body: true,
            capture_response_body: false,
            max_body_size: DEFAULT_MAX_BODY_SIZE,
            redacted_headers: vec![
                "authorization".to_string(),
                "cookie".to_string(),
                "proxy-authorization".to_string(),
                "set-cookie".to_string(),
            ],
            redacted_query_params: vec![
                "access_token".to_string(),
                "api_key".to_string(),
                "key".to_string(),
                "password".to_string(),
                "refresh_token".to_string(),
                "secret".to_string(),
                "token".to_string(),
            ],
        }
    }
}

/// A small wrapper around `reqwest::Client` that captures calls sent through it.
///
/// Use this when you want to instrument selected call sites without adding `reqwest-middleware`.
/// For broad capture across an existing client, prefer `AllureReqwestMiddleware` with the
/// `middleware` feature.
#[derive(Clone)]
pub struct AllureReqwestClient {
    client: reqwest::Client,
    allure: AllureFacade,
    options: CaptureOptions,
}

impl AllureReqwestClient {
    /// Creates a client using a new `reqwest::Client`.
    pub fn new(allure: AllureFacade) -> Self {
        Self::with_client(reqwest::Client::new(), allure)
    }

    /// Creates a wrapper around an existing `reqwest::Client`.
    pub fn with_client(client: reqwest::Client, allure: AllureFacade) -> Self {
        Self {
            client,
            allure,
            options: CaptureOptions::default(),
        }
    }

    /// Replaces capture options.
    pub fn with_options(mut self, options: CaptureOptions) -> Self {
        self.options = options;
        self
    }

    /// Returns the wrapped reqwest client.
    pub fn inner(&self) -> &reqwest::Client {
        &self.client
    }

    /// Creates a request builder for an arbitrary method.
    pub fn request(&self, method: Method, url: impl reqwest::IntoUrl) -> RequestBuilder {
        self.client.request(method, url)
    }

    /// Creates a `GET` request builder.
    pub fn get(&self, url: impl reqwest::IntoUrl) -> RequestBuilder {
        self.client.get(url)
    }

    /// Creates a `POST` request builder.
    pub fn post(&self, url: impl reqwest::IntoUrl) -> RequestBuilder {
        self.client.post(url)
    }

    /// Creates a `PUT` request builder.
    pub fn put(&self, url: impl reqwest::IntoUrl) -> RequestBuilder {
        self.client.put(url)
    }

    /// Creates a `PATCH` request builder.
    pub fn patch(&self, url: impl reqwest::IntoUrl) -> RequestBuilder {
        self.client.patch(url)
    }

    /// Creates a `DELETE` request builder.
    pub fn delete(&self, url: impl reqwest::IntoUrl) -> RequestBuilder {
        self.client.delete(url)
    }

    /// Creates a `HEAD` request builder.
    pub fn head(&self, url: impl reqwest::IntoUrl) -> RequestBuilder {
        self.client.head(url)
    }

    /// Builds and executes a request builder while capturing an HTTP exchange.
    pub async fn send(&self, request: RequestBuilder) -> reqwest::Result<Response> {
        self.execute(request.build()?).await
    }

    /// Executes a request while capturing an HTTP exchange.
    pub async fn execute(&self, request: Request) -> reqwest::Result<Response> {
        execute_with_capture(
            &self.allure,
            &self.options,
            request,
            "reqwest::Error",
            |request| self.client.execute(request),
        )
        .await
    }
}

async fn execute_with_capture<F, E>(
    allure: &AllureFacade,
    options: &CaptureOptions,
    request: Request,
    error_name: &str,
    execute: impl FnOnce(Request) -> F,
) -> Result<Response, E>
where
    F: std::future::Future<Output = Result<Response, E>>,
    E: From<reqwest::Error> + std::fmt::Display,
{
    let start = now_millis();
    let mut exchange = exchange_from_request(&request, options);
    exchange.start = Some(start);

    let result = execute(request).await;
    let stop = now_millis();
    exchange.stop = Some(stop);

    match result {
        Ok(response) => {
            let response = capture_response(response, &mut exchange, options, error_name)
                .await
                .map_err(E::from)?;
            attach_exchange(allure, options, exchange);
            Ok(response)
        }
        Err(error) => {
            exchange.error = Some(HttpExchangeError {
                name: Some(error_name.to_string()),
                message: Some(error.to_string()),
                stack: None,
            });
            attach_exchange(allure, options, exchange);
            Err(error)
        }
    }
}

fn exchange_from_request(request: &Request, options: &CaptureOptions) -> HttpExchange {
    let mut exchange = HttpExchange {
        schema_version: 1,
        request: HttpExchangeRequest::new(request.method().as_str(), request.url().to_string()),
        response: None,
        error: None,
        start: None,
        stop: None,
    };
    exchange.request.http_version = Some(version_to_string(request.version()));
    exchange.request.headers = non_empty(headers_to_name_values(
        request.headers(),
        &options.redacted_headers,
    ));
    exchange.request.query = non_empty(query_to_name_values(
        request.url(),
        &options.redacted_query_params,
    ));
    if options.capture_request_body {
        exchange.request.body = request
            .body()
            .and_then(|body| body.as_bytes())
            .map(|bytes| {
                body_from_bytes(
                    bytes,
                    content_type(request.headers()),
                    options.max_body_size,
                )
            });
    }
    exchange
}

async fn capture_response(
    response: Response,
    exchange: &mut HttpExchange,
    options: &CaptureOptions,
    error_name: &str,
) -> reqwest::Result<Response> {
    let status = response.status();
    let version = response.version();
    let headers = response.headers().clone();
    let content_type = content_type(&headers);

    let mut captured = HttpExchangeResponse {
        status: Some(status.as_u16()),
        status_text: status.canonical_reason().map(str::to_string),
        http_version: Some(version_to_string(version)),
        headers: non_empty(headers_to_name_values(&headers, &options.redacted_headers)),
        ..Default::default()
    };

    if !options.capture_response_body {
        exchange.response = Some(captured);
        return Ok(response);
    }

    match response.bytes().await {
        Ok(bytes) => {
            captured.body = Some(body_from_bytes(
                bytes.as_ref(),
                content_type,
                options.max_body_size,
            ));
            exchange.response = Some(captured);
            Ok(response_from_parts(status, version, headers, bytes))
        }
        Err(error) => {
            exchange.response = Some(captured);
            exchange.error = Some(HttpExchangeError {
                name: Some(error_name.to_string()),
                message: Some(error.to_string()),
                stack: None,
            });
            Err(error)
        }
    }
}

fn attach_exchange(allure: &AllureFacade, options: &CaptureOptions, exchange: HttpExchange) {
    if let Some(name) = &options.attachment_name {
        allure.http_exchange_named(name.clone(), exchange);
    } else {
        allure.http_exchange(exchange);
    }
}

fn response_from_parts(
    status: reqwest::StatusCode,
    version: Version,
    headers: HeaderMap,
    bytes: bytes::Bytes,
) -> Response {
    let mut response = http::Response::builder()
        .status(status)
        .version(version)
        .body(reqwest::Body::from(bytes))
        .expect("captured reqwest response parts should build an http response");
    *response.headers_mut() = headers;
    Response::from(response)
}

fn body_from_bytes(
    bytes: &[u8],
    content_type: Option<String>,
    max_body_size: usize,
) -> HttpExchangeBody {
    let captured_len = bytes.len().min(max_body_size);
    let captured = &bytes[..captured_len];
    let truncated = bytes.len() > captured_len;

    let (encoding, value) = match std::str::from_utf8(captured) {
        Ok(text) => (HttpExchangeBodyEncoding::Utf8, text.to_string()),
        Err(_) => (HttpExchangeBodyEncoding::Base64, STANDARD.encode(captured)),
    };

    HttpExchangeBody {
        content_type,
        encoding: Some(encoding),
        value: Some(value),
        size: Some(bytes.len() as u64),
        truncated: Some(truncated),
        ..Default::default()
    }
}

fn headers_to_name_values(
    headers: &HeaderMap,
    redacted_headers: &[String],
) -> Vec<HttpExchangeNameValue> {
    headers
        .iter()
        .map(|(name, value)| {
            let value = if is_redacted_name(name.as_str(), redacted_headers) {
                HTTP_EXCHANGE_REDACTED_VALUE.to_string()
            } else {
                value
                    .to_str()
                    .map(str::to_string)
                    .unwrap_or_else(|_| STANDARD.encode(value.as_bytes()))
            };
            HttpExchangeNameValue::new(name.as_str(), value)
        })
        .collect()
}

fn query_to_name_values(url: &Url, redacted_query_params: &[String]) -> Vec<HttpExchangeNameValue> {
    url.query_pairs()
        .map(|(name, value)| {
            let value = if is_redacted_name(name.as_ref(), redacted_query_params) {
                HTTP_EXCHANGE_REDACTED_VALUE.to_string()
            } else {
                value.into_owned()
            };
            HttpExchangeNameValue::new(name.into_owned(), value)
        })
        .collect()
}

fn content_type(headers: &HeaderMap) -> Option<String> {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

fn non_empty<T>(values: Vec<T>) -> Option<Vec<T>> {
    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

fn is_redacted_name(name: &str, redacted_names: &[String]) -> bool {
    let name = normalize_name(name);
    redacted_names.iter().any(|redacted| redacted == &name)
}

fn normalize_name(name: impl AsRef<str>) -> String {
    name.as_ref().to_ascii_lowercase()
}

fn version_to_string(version: Version) -> String {
    match version {
        Version::HTTP_09 => "HTTP/0.9",
        Version::HTTP_10 => "HTTP/1.0",
        Version::HTTP_11 => "HTTP/1.1",
        Version::HTTP_2 => "HTTP/2",
        Version::HTTP_3 => "HTTP/3",
        _ => return format!("{version:?}"),
    }
    .to_string()
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

#[cfg(feature = "middleware")]
/// `reqwest_middleware` integration for automatic HTTP exchange capture.
pub mod middleware {
    use super::*;

    /// Middleware that captures every request passing through `reqwest_middleware`.
    ///
    /// This is the recommended integration style for existing clients and suites that want broad
    /// HTTP evidence with minimal call-site changes.
    #[derive(Clone)]
    pub struct AllureReqwestMiddleware {
        allure: AllureFacade,
        options: CaptureOptions,
    }

    impl AllureReqwestMiddleware {
        /// Creates middleware that records exchanges through the provided Allure facade.
        pub fn new(allure: AllureFacade) -> Self {
            Self {
                allure,
                options: CaptureOptions::default(),
            }
        }

        /// Configures capture behavior for subsequent middleware requests.
        pub fn with_options(mut self, options: CaptureOptions) -> Self {
            self.options = options;
            self
        }
    }

    #[async_trait::async_trait]
    impl reqwest_middleware::Middleware for AllureReqwestMiddleware {
        async fn handle(
            &self,
            req: Request,
            extensions: &mut http::Extensions,
            next: reqwest_middleware::Next<'_>,
        ) -> reqwest_middleware::Result<Response> {
            execute_with_capture(
                &self.allure,
                &self.options,
                req,
                "reqwest_middleware::Error",
                |req| next.run(req, extensions),
            )
            .await
        }
    }
}

#[cfg(feature = "middleware")]
pub use middleware::AllureReqwestMiddleware;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
