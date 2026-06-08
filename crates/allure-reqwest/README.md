# allure-reqwest

`allure-reqwest` captures `reqwest` HTTP calls as Allure HTTP Exchange attachments.

## Add the crate

```bash
cargo add allure-reqwest --dev
```

## Which API should I use?

Prefer middleware when you want broad capture with minimal call-site changes. Middleware runs around
each request made through a `reqwest_middleware::ClientWithMiddleware`, so it can record the request,
response, timing, and errors automatically while your application continues to use a reqwest-style
client.

Use the wrapper client when you want the smallest dependency surface or you are instrumenting only a
few calls. The wrapper does not require `reqwest-middleware`, but calls must be sent through
`AllureReqwestClient::send` or `AllureReqwestClient::execute`.

## Recommended: middleware

Enable the `middleware` feature and attach `AllureReqwestMiddleware` to a middleware client.

```toml
[dependencies]
allure-reqwest = { version = "1", features = ["middleware"] }
reqwest = "0.12"
reqwest-middleware = "0.4"
```

```rust
use allure_reqwest::AllureReqwestMiddleware;
use reqwest_middleware::ClientBuilder;

async fn create_order(allure: allure_rust_commons::AllureFacade) -> reqwest::Result<()> {
    let client = ClientBuilder::new(reqwest::Client::new())
        .with(AllureReqwestMiddleware::new(allure))
        .build();

    let response = client
        .post("https://api.example.com/v1/orders")
        .header("content-type", "application/json")
        .body(r#"{"name":"demo"}"#)
        .send()
        .await?;

    response.error_for_status()?;
    Ok(())
}
```

## Alternative: wrapper client

Create an `AllureReqwestClient` from an active `AllureFacade`, build requests through the wrapper,
and send them with `send`.

```rust
use allure_reqwest::AllureReqwestClient;

async fn create_order(allure: allure_rust_commons::AllureFacade) -> reqwest::Result<()> {
    let client = AllureReqwestClient::new(allure);
    let response = client
        .send(
            client
                .post("https://api.example.com/v1/orders")
                .header("content-type", "application/json")
                .body(r#"{"name":"demo"}"#),
        )
        .await?;

    response.error_for_status()?;
    Ok(())
}
```

## Capture options

By default the integration captures method, URL, query, headers, status, timing, and transport
errors. In-memory request bodies are captured up to a bounded size. Response body capture is opt-in.

```rust
use allure_reqwest::{AllureReqwestClient, CaptureOptions};

let client = AllureReqwestClient::new(allure).with_options(
    CaptureOptions::default().with_response_body_capture(64 * 1024),
);
```
