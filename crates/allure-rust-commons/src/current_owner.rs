use crate::{
    facade::AllureFacade,
    http_exchange::{HttpExchange, HTTP_EXCHANGE_ATTACHMENT_NAME},
};

pub(crate) fn add_attachment(
    allure: &AllureFacade,
    name: impl Into<String>,
    content_type: impl Into<String>,
    body: impl AsRef<[u8]>,
) {
    if let Some(lifecycle) = allure.lifecycle() {
        lifecycle.add_attachment(name, content_type, body.as_ref());
    }
}

pub(crate) fn add_http_exchange(allure: &AllureFacade, exchange: HttpExchange) {
    add_http_exchange_named(allure, HTTP_EXCHANGE_ATTACHMENT_NAME, exchange);
}

pub(crate) fn add_http_exchange_named(
    allure: &AllureFacade,
    name: impl Into<String>,
    exchange: HttpExchange,
) {
    if let Some(lifecycle) = allure.lifecycle() {
        lifecycle.add_http_exchange_named(name, exchange);
    }
}
