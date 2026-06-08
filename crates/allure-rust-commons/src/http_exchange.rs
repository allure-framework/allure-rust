use serde::{Serialize, Serializer};

pub const HTTP_EXCHANGE_ATTACHMENT_MIME: &str = "application/vnd.allure.http+json";
pub const HTTP_EXCHANGE_ATTACHMENT_EXTENSION: &str = ".httpexchange";
pub const HTTP_EXCHANGE_REDACTED_VALUE: &str = "__ALLURE_REDACTED__";

pub(crate) const HTTP_EXCHANGE_ATTACHMENT_NAME: &str = "HTTP Exchange";
pub(crate) const HTTP_EXCHANGE_SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpExchange {
    pub schema_version: u8,
    pub request: HttpExchangeRequest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<HttpExchangeResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<HttpExchangeError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<i64>,
}

impl HttpExchange {
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpExchangeRequest {
    pub method: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookies: Option<Vec<HttpExchangeCookie>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<HttpExchangeNameValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<Vec<HttpExchangeNameValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<HttpExchangeBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trailers: Option<Vec<HttpExchangeNameValue>>,
}

impl HttpExchangeRequest {
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

#[derive(Debug, Clone, Default, Serialize)]
pub struct HttpExchangeNameValue {
    pub name: String,
    pub value: String,
}

impl HttpExchangeNameValue {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpExchangeCookie {
    pub name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_age: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub same_site: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secure: Option<bool>,
}

#[derive(Debug, Clone)]
pub enum HttpExchangeBodyEncoding {
    Utf8,
    Base64,
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

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpExchangeBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<HttpExchangeBodyEncoding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<Vec<HttpExchangeNameValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parts: Option<Vec<HttpExchangeBodyPart>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<HttpExchangeStream>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpExchangeBodyPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<HttpExchangeNameValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<HttpExchangeBodyEncoding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpExchangeStream {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub stream_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub complete: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_count: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpExchangeInformationalResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<HttpExchangeNameValue>>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpExchangeResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookies: Option<Vec<HttpExchangeCookie>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<HttpExchangeNameValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<HttpExchangeBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trailers: Option<Vec<HttpExchangeNameValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub informational_responses: Option<Vec<HttpExchangeInformationalResponse>>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpExchangeError {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
}

#[cfg(test)]
#[path = "http_exchange_tests.rs"]
mod http_exchange_tests;
