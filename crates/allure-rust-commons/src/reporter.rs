//! Exact-owner helpers for reporter and adapter internals.

use crate::{current_owner, facade::AllureFacade, http_exchange::HttpExchange};

/// Attaches bytes to the exact current owner without creating a wrapper step.
pub fn attachment(
    allure: &AllureFacade,
    name: impl Into<String>,
    content_type: impl Into<String>,
    body: impl AsRef<[u8]>,
) {
    current_owner::add_attachment(allure, name, content_type, body);
}

/// Attaches an HTTP exchange to the exact current owner without creating a wrapper step.
pub fn http_exchange(allure: &AllureFacade, exchange: HttpExchange) {
    current_owner::add_http_exchange(allure, exchange);
}

/// Attaches a named HTTP exchange to the exact current owner without creating a wrapper step.
pub fn http_exchange_named(allure: &AllureFacade, name: impl Into<String>, exchange: HttpExchange) {
    current_owner::add_http_exchange_named(allure, name, exchange);
}
