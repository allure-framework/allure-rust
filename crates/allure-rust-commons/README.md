# allure-rust-commons

`allure-rust-commons` provides the core runtime, lifecycle, model, writer, and facade APIs
used to produce Allure-compatible result files.

Use this crate when you want to build your own adapter for a custom test runner or framework.

## Add the crate

```bash
cargo add allure-rust-commons
```

## Basic usage

Create a results writer, initialize a runtime, then start and stop a test through the lifecycle
or the facade API.

```rust
use allure_rust_commons::{AllureFacade, AllureRuntime, FileSystemResultsWriter, Status};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let writer = FileSystemResultsWriter::new("target/allure-results")?;
    let runtime = AllureRuntime::new(writer);
    let lifecycle = runtime.lifecycle();
    let allure = AllureFacade::with_lifecycle(lifecycle);

    allure.start_test_with_full_name("login_works", "auth::login_works");
    allure.feature("Authentication");
    allure.parameter("browser", "firefox");
    allure.stage("open login page");
    allure.log_step("open login page");
    allure.attachment("request.json", "application/json", br#"{"ok":true}"#);
    allure.end_test(Status::Passed, None);

    Ok(())
}
```

## Thread-bound convenience functions

Adapters can bind an `AllureFacade` to the current thread while a test runs. After that, users can
import framework-neutral functions from this crate instead of passing an `AllureFacade` through
every helper.

```rust
use allure_rust_commons::{
    feature, log_step, parameter, push_current_allure, stage, AllureFacade, AllureRuntime,
    FileSystemResultsWriter, Status,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let writer = FileSystemResultsWriter::new("target/allure-results")?;
    let runtime = AllureRuntime::new(writer);
    let allure = AllureFacade::with_lifecycle(runtime.lifecycle());

    allure.start_test_with_full_name("login_works", "auth::login_works");
    let _current = push_current_allure(&allure);

    feature("Authentication");
    parameter("browser", "firefox");
    stage("open login page");
    log_step("open login page");

    allure.end_test(Status::Passed, None);
    Ok(())
}
```

## Macro-free cargo test usage

When attribute macros are not allowed, wrap a regular Rust `#[test]` with `allure::test`. The
test name is inferred from the current cargo-test thread, and the closure gets the same
thread-bound runtime context used by the convenience functions.

```rust
use allure_rust_commons as allure;

#[test]
fn login_works() {
    allure::test(|| {
        allure::feature("Authentication");
        allure::parameter("browser", "firefox");
        allure::stage("prepare data");
        let user = "alice";
        allure::stage("submit credentials");
        allure::step("send request", || {
            allure::attachment("request.json", "application/json", br#"{"user":"alice"}"#);
        });
        allure::stage("verify result");
        assert_eq!(user, "alice");
    });
}
```

Use `test_named` for a custom display name, or `test_with(TestOptions::new(...), || { ... })`
when an adapter or helper can provide exact metadata such as full name, title path, manifest dir,
module path, labels, or IDs. Async tests can use `test_async`, `test_named_async`, or
`test_with_async`.

Adapters that need Rust test-style outcome handling can use `test_with_outcome` or
`test_with_outcome_async`. These wrappers report values implementing `AllureTestOutcome`, including
`()`, nested `Result<T, E>` where `T` is also an Allure outcome, and `ExitCode`, while still
returning the original value to the test harness.

## Runtime facade

The thread-bound facade exports framework-neutral helpers for the common Allure concepts:

- Metadata: `display_name`, `description`, `description_html`, `history_id`, `test_case_id`,
  `id`, `allure_id`
- Labels and links: `label`, `labels`, `epic`, `feature`, `story`, `suite`, `parent_suite`,
  `sub_suite`, `owner`, `severity`, `tag`, `tags`, `link`, `links`, `issue`, `tms`
- Parameters: `parameter`, `parameter_excluded`, `parameter_mode`, `parameter_with_options`
- Steps: `log_step`, `log_step_with`, `step`, `stage`
- Evidence: `attachment`, `attachment_path`, `attach_trace`, `http_exchange`, and
  `http_exchange_named`
- Run-level diagnostics: `global_attachment`, `global_attachment_path`, `global_error`,
  `global_error_with_trace`

Runtime attachment and HTTP exchange helpers create an ordered evidence step by default. Adapter
internals that need exact active-owner attachment placement can use the reporter module:

```rust
use allure_rust_commons::{reporter, AllureFacade, HttpExchange};

fn attach_from_adapter(allure: &AllureFacade) {
    reporter::attachment(allure, "trace.txt", "text/plain", "raw trace");
    reporter::http_exchange(allure, HttpExchange::new("GET", "https://example.invalid"));
}
```

## HTTP exchange attachments

Use `HttpExchange` when an integration captures request/response evidence for Allure viewers or
API coverage tools.

```rust
use allure_rust_commons::{
    AllureFacade, HttpExchange, HttpExchangeBody, HttpExchangeBodyEncoding, HttpExchangeResponse,
};

fn attach_order_exchange(allure: &AllureFacade) {
    let mut exchange = HttpExchange::new("POST", "https://api.example.com/v1/orders");
    exchange.request.body = Some(HttpExchangeBody {
        content_type: Some("application/json".to_string()),
        encoding: Some(HttpExchangeBodyEncoding::Utf8),
        value: Some(r#"{"name":"demo"}"#.to_string()),
        size: Some(15),
        truncated: Some(false),
        ..Default::default()
    });
    exchange.response = Some(HttpExchangeResponse {
        status: Some(201),
        status_text: Some("Created".to_string()),
        ..Default::default()
    });

    allure.http_exchange(exchange);
}
```

## Common building blocks

- `AllureRuntime`: owns the configured results writer.
- `AllureLifecycle`: manages in-progress tests and steps.
- `AllureFacade`: ergonomic helper for labels, links, parameters, steps, attachments, identity,
  and run-level diagnostics.
- Macro-free test runners: `test`, `test_named`, `test_with`, and async equivalents.
- Thread-bound functions delegate to the current facade set by `push_current_allure`.
- `FileSystemResultsWriter`: persists JSON result files and attachments.
- `config`: a process-wide env/runtime config singleton plus cached Cargo metadata labels and
  title-path helpers.
- `model`: raw Allure data structures for custom integrations.

## Configuration Helpers

`AllureLifecycle::start_test_case` automatically adds global labels from `ALLURE_LABEL_*` and
`allure.label.*` environment variables. Integrations can call `apply_config_labels` to add labels
from `[package.metadata.allure]` in `Cargo.toml`, and `apply_common_runtime_labels` to add shared
runtime labels such as `language`, `host`, and `thread`.

Environment-derived settings such as `ALLURE_RESULTS_DIR`, global labels, host override, thread
override, and assertion-logging override are loaded once per process through `global_config()`.
Assertion logging is enabled by default and can be disabled with `ALLURE_LOG_ASSERTS=false` or
`[package.metadata.allure] log_asserts = false`. Cargo metadata is cached separately per manifest
directory.

## Output location

The writer creates the target directory if it does not already exist:

```rust
let writer = FileSystemResultsWriter::new("target/allure-results")?;
```
