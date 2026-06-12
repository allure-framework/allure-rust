//! Types for Allure HTTP Exchange attachments.

use serde::{Serialize, Serializer};

/// MIME type for Allure HTTP Exchange attachments.
pub const HTTP_EXCHANGE_ATTACHMENT_MIME: &str = "application/vnd.allure.http+json";
/// File extension used for HTTP Exchange attachment payloads.
pub const HTTP_EXCHANGE_ATTACHMENT_EXTENSION: &str = ".httpexchange";
/// Placeholder value used when a captured HTTP value is redacted.
pub const HTTP_EXCHANGE_REDACTED_VALUE: &str = "__ALLURE_REDACTED__";

pub(crate) const HTTP_EXCHANGE_ATTACHMENT_NAME: &str = "HTTP Exchange";
pub(crate) const HTTP_EXCHANGE_SCHEMA_VERSION: u8 = 1;

/// Complete HTTP exchange payload stored in an attachment.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpExchange {
    /// Schema version of the HTTP exchange payload.
    pub schema_version: u8,
    /// Captured request data.
    pub request: HttpExchangeRequest,
    /// Optional captured response data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<HttpExchangeResponse>,
    /// Optional transport or protocol error data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<HttpExchangeError>,
    /// Request start timestamp in milliseconds since the Unix epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<i64>,
    /// Request stop timestamp in milliseconds since the Unix epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<i64>,
}

impl HttpExchange {
    /// Creates a minimal HTTP exchange with method and URL.
    pub fn new(method: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            schema_version: HTTP_EXCHANGE_SCHEMA_VERSION,
            request: HttpExchangeRequest::new(method, url),
            response: None,
            error: None,
            start: None,
            stop: None,
        }
    }
}

/// Captured HTTP request data.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpExchangeRequest {
    /// HTTP method.
    pub method: String,
    /// Request URL.
    pub url: String,
    /// HTTP protocol version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_version: Option<String>,
    /// Request cookies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookies: Option<Vec<HttpExchangeCookie>>,
    /// Request headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<HttpExchangeNameValue>>,
    /// Parsed query parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<Vec<HttpExchangeNameValue>>,
    /// Request body.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<HttpExchangeBody>,
    /// Request trailers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trailers: Option<Vec<HttpExchangeNameValue>>,
}

impl HttpExchangeRequest {
    /// Creates a minimal HTTP exchange request with method and URL.
    pub fn new(method: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            url: url.into(),
            http_version: None,
            cookies: None,
            headers: None,
            query: None,
            body: None,
            trailers: None,
        }
    }
}

/// Name/value pair used for headers, query parameters, and form values.
#[derive(Debug, Clone, Default, Serialize)]
pub struct HttpExchangeNameValue {
    /// Field name.
    pub name: String,
    /// Field value.
    pub value: String,
}

impl HttpExchangeNameValue {
    /// Creates a name/value pair.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

/// Captured HTTP cookie.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpExchangeCookie {
    /// Cookie name.
    pub name: String,
    /// Cookie value.
    pub value: String,
    /// Cookie domain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Cookie expiration value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires: Option<String>,
    /// Whether the cookie is HTTP-only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_only: Option<bool>,
    /// Cookie max-age in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_age: Option<i64>,
    /// Cookie path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// SameSite value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub same_site: Option<String>,
    /// Whether the cookie is marked secure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secure: Option<bool>,
}

/// Encoding used for an HTTP body value.
#[derive(Debug, Clone)]
pub enum HttpExchangeBodyEncoding {
    /// UTF-8 text.
    Utf8,
    /// Base64-encoded bytes.
    Base64,
    /// Custom encoding value.
    Other(String),
}

impl Serialize for HttpExchangeBodyEncoding {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = match self {
            Self::Utf8 => "utf8",
            Self::Base64 => "base64",
            Self::Other(value) => value.as_str(),
        };
        serializer.serialize_str(value)
    }
}

/// Captured HTTP body.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpExchangeBody {
    /// Body content type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// Body value encoding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<HttpExchangeBodyEncoding>,
    /// Captured body value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Original body size in bytes when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// Whether the captured value was truncated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
    /// Parsed form fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<Vec<HttpExchangeNameValue>>,
    /// Multipart body parts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parts: Option<Vec<HttpExchangeBodyPart>>,
    /// Stream metadata when the body is not captured as a value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<HttpExchangeStream>,
}

/// Captured multipart body part.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpExchangeBodyPart {
    /// Part name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Uploaded file name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    /// Part headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<HttpExchangeNameValue>>,
    /// Part content type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// Part value encoding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<HttpExchangeBodyEncoding>,
    /// Captured part value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Original part size in bytes when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// Whether the captured part value was truncated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
}

/// Metadata for streamed HTTP body content.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpExchangeStream {
    /// Stream type.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub stream_type: Option<String>,
    /// Whether the stream completed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub complete: Option<bool>,
    /// Number of chunks observed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_count: Option<u64>,
}

/// Captured informational HTTP response.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpExchangeInformationalResponse {
    /// Response status code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    /// Response status text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_text: Option<String>,
    /// Response headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<HttpExchangeNameValue>>,
}

/// Captured HTTP response.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpExchangeResponse {
    /// Response status code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    /// Response status text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_text: Option<String>,
    /// HTTP protocol version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_version: Option<String>,
    /// Response cookies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookies: Option<Vec<HttpExchangeCookie>>,
    /// Response headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<HttpExchangeNameValue>>,
    /// Response body.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<HttpExchangeBody>,
    /// Response trailers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trailers: Option<Vec<HttpExchangeNameValue>>,
    /// Informational responses received before the final response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub informational_responses: Option<Vec<HttpExchangeInformationalResponse>>,
}

/// Captured HTTP transport or protocol error.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpExchangeError {
    /// Error name or class.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Error message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Error stack trace or diagnostic text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
}

#[cfg(test)]
#[path = "http_exchange_tests.rs"]
mod http_exchange_tests;
