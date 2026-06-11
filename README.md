# Allure Rust

> Official Allure Framework libraries for generating Allure Report results in Rust.

[<img src="https://allurereport.org/public/img/allure-report.svg" height="85px" alt="Allure Report logo" align="right" />](https://allurereport.org "Allure Report")

- Learn more about Allure Report at https://allurereport.org
- 📚 [Documentation](https://allurereport.org/docs/) – discover official documentation for Allure Report
- ❓ [Questions and Support](https://github.com/orgs/allure-framework/discussions/categories/questions-support) – get help from the team and community
- 📢 [Official annoucements](https://github.com/orgs/allure-framework/discussions/categories/announcements) – be in touch with the latest updates
- 💬 [General Discussion ](https://github.com/orgs/allure-framework/discussions/categories/general-discussion) – engage in casual conversations, share insights and ideas with the community

---

## Overview

`allure-rust` is the Rust implementation of the Allure Framework. It provides primitives to build
Allure-compatible results and a ready-to-use integration for `cargo test`.

## Packages

This workspace currently includes:

- `allure-rust-commons`: core runtime model, lifecycle, writer, and facade APIs.
- `allure-reqwest`: `reqwest` integration for Allure HTTP Exchange attachments.
- `allure-cargotest`: test integration helper and reporter for `cargo test`.
- `allure-test-macros`: procedural macro crate that provides `#[allure_test]`, `#[step]`,
  and `#[log_asserts]`.

## Basic installation

### 1. Add a Rust integration crate

For regular Rust tests with automatic lifecycle wiring:

```bash
cargo add allure-cargotest --dev
```

For building your own integration/framework adapter:

```bash
cargo add allure-rust-commons
```

### 2. Generate results in your test run

By default, results are written to `target/allure-results`.
You can override it with:

```bash
ALLURE_RESULTS_DIR=./allure-results cargo test
```

### 3. Generate a report with Allure 3

After tests finish and `*-result.json` files are present in your results directory,
use the Allure 3 CLI to generate and open the report:

```bash
allure generate ./target/allure-results --output ./target/allure-report --clean
allure open ./target/allure-report
```

> If you installed Allure 3 through Node tooling, use the equivalent `npx` command
> provided by your installation method.

## Supported versions and platforms

- Rust: stable toolchain (workspace uses Rust 2021 edition).
- OS: cross-platform by design (file-system based output, no OS-specific APIs).
- CI coverage in this repository currently runs on `ubuntu-latest`.

## Runtime features

Implemented runtime facade APIs include:

- **Test lifecycle**: `start_test_case`, `stop_test_case` (plus compatibility aliases `start_test`, `start_test_with_full_name`, `end_test`)
- **Descriptions**: `description`, `description_html`
- **Labels and identity**: `label`, `labels`, `epic`, `feature`, `story`, `suite`,
  `parent_suite`, `sub_suite`, `owner`, `severity`, `layer`, `tag`, `tags`, `id`,
  `allure_id`, `display_name`, `history_id`, `test_case_id`
- **Links**: `link`, `links`, `issue`, `tms`
- **Parameters**: `parameter`, `parameter_excluded`, `parameter_mode`,
  `parameter_with_options`
- **Attachments**: `attachment`, `attachment_path`, `attach_trace`, and global diagnostics helpers
- **HTTP exchanges**: `HttpExchange` in commons, plus `allure-reqwest` for `reqwest` clients.
  Runtime helpers wrap exchanges as ordered evidence steps by default; exact-owner helpers live in
  `allure_rust_commons::reporter` for reporter/adapter internals.
- **Steps and stages**: `log_step`, `log_step_with`, `step`, `stage`, `#[step]`
- **Assertion logging**: `assert!`, `assert_eq!`, and related standard assertion steps are logged
  by default and can be disabled with `log_asserts` configuration

## Quick example

### `allure-cargotest`

```rust
use allure_cargotest::{allure_test, step};
use allure_rust_commons::{feature, log_step, parameter, stage};

#[allure_test]
#[test]
fn test_with_allure() {
    feature("Authentication");
    parameter("browser", "firefox");

    stage("open login page");
    open_login_page();
    log_step("login page opened");
}

#[step]
fn open_login_page() {
    // your step implementation
}
```

The `#[allure_test]` attribute initializes a reporter automatically (using
`ALLURE_RESULTS_DIR` or `target/allure-results`) and wraps the test lifecycle.

For environments where attribute macros are not allowed, use the macro-free runtime from commons:

```rust
use allure_rust_commons as allure;

#[test]
fn test_with_allure_runtime() {
    allure::test(|| {
        allure::feature("Authentication");
        allure::stage("open login page");
        allure::log_step("open login page");
    });
}
```

Async tests can compose `#[allure_test]` with a runtime-specific test macro such as
`#[tokio::test]`. Add and configure Tokio in your test crate; `allure-cargotest` does not depend
on Tokio.

```rust
use allure_cargotest::allure_test;

#[allure_test]
#[tokio::test]
async fn async_test_with_allure() {
    allure.feature("Async API");
    tokio::task::yield_now().await;
    allure.parameter("runtime", "tokio");
}
```

Allure context is available in the root async test body and awaited helpers. Implicit propagation
into independently spawned tasks is not guaranteed.

`#[allure_test]` also derives synthetic suite labels from the Rust module path when a full
name is available. A single module segment becomes `suite`; with two or more segments, the first
becomes `parentSuite`, the second becomes `suite`, and any remaining module segments are joined
into `subSuite`. If your test calls `allure.parent_suite(...)`, `allure.suite(...)`, or
`allure.sub_suite(...)`, those explicit labels override the synthetic defaults for the same label
name.

## Development

### `allure-rust-commons` module documentation

`allure-rust-commons` is the foundation layer that other integrations build on.
It exports these main building blocks:

- **Model (`model`)**: Allure data structures (`TestResult`, `StepResult`, labels,
  links, parameters, attachments, statuses).
- **Lifecycle (`lifecycle`)**:
  - `AllureRuntime`: owns a results writer and creates per-execution lifecycles.
  - `AllureLifecycle`: manages active test state, nested steps, metadata, and final write.
- **Writer (`writer`)**:
  - `FileSystemResultsWriter`: writes result JSON and attachment files into an output directory.
- **Facade (`facade`)**:
  - `AllureFacade`: ergonomic API for tests/framework adapters.
  - Macro-free test runners such as `test`, `test_named`, and `test_with`.
  - Thread-bound helpers such as `feature`, `parameter`, `log_step`, `attachment`, and `step`.
  - global `allure()` accessor backed by `OnceLock`.
- **Config (`config`)**:
  - `global_config()`: process-wide env/runtime settings loaded once.
  - Cargo metadata config cached per manifest directory.

This separation lets you choose either:

1. A high-level API (`AllureFacade`) for minimal integration effort.
2. A low-level API (`AllureLifecycle` + model) for custom control.

## Create your own integration with `allure-rust-commons`

To integrate Allure with another Rust test runner or framework:

1. **Initialize a writer and runtime** once per process (or execution session).
2. **Create a lifecycle** for test execution scope.
3. **Wrap framework hooks/events**:
   - on test start -> `start_test_case(...)`
   - during test -> labels/links/parameters/steps/attachments
   - on test end -> `stop_test_case(status, details)`
4. **Map framework statuses** to Allure statuses (`Passed`, `Failed`, `Broken`, `Skipped`).
5. **Persist artifacts** via `add_attachment(...)` when your framework emits logs/files.

Minimal sketch:

```rust
use allure_rust_commons::{AllureRuntime, FileSystemResultsWriter, Status};

let writer = FileSystemResultsWriter::new("target/allure-results")?;
let runtime = AllureRuntime::new(writer);
let lifecycle = runtime.lifecycle();

lifecycle.start_test_case(
    allure_rust_commons::StartTestCaseParams::new("my test").with_full_name("suite::my test")
);
// ... update metadata, steps, and attachments during execution ...
lifecycle.stop_test_case(Status::Passed, None);
```

For convenience APIs, wrap the lifecycle in `AllureFacade::with_lifecycle(...)` and expose
framework-specific helpers similar to `#[allure_test]` from `allure-cargotest`. Adapters can also
call `push_current_allure(&facade)` while a test runs so users can import thread-bound functions
directly from `allure-rust-commons`.

## Why is there a separate `allure-test-macros` crate?

Rust requires attribute procedural macros to be compiled from a `proc-macro` crate.
A `proc-macro` crate cannot serve as a regular runtime library crate at the same
time, so runtime/reporter APIs stay in `allure-cargotest` and macro implementation
lives in `allure-test-macros`.

From a user perspective this remains a single entrypoint because
`allure-cargotest` re-exports `#[allure_test]`, `#[step]`, and `#[log_asserts]`.


## Community

- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Contributing Guide](CONTRIBUTING.md)
- [Security Policy](SECURITY.md)
