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
- `allure-cargotest`: test integration helper and reporter for `cargo test`.
- `allure-test-macros`: procedural macro crate that provides `#[allure_test]` and `#[step]`.

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
- **Labels**: `label`, `labels`, `epic`, `feature`, `story`, `suite`, `parent_suite`,
  `sub_suite`, `owner`, `severity`, `layer`, `tag`, `tags`, `id`
- **Links**: `link`, `links`, `issue`, `tms`
- **Parameters**: `parameter`
- **Attachments**: `attachment`
- **Steps**: `step`, `log_step`, `#[step]` with automatic stop via `StepGuard`

## Quick example

### `allure-cargotest`

```rust
use allure_cargotest::{allure_test, step};

#[allure_test]
#[test]
fn test_with_allure() {
    allure.epic("Web interface");
    allure.feature("Authentication");
    allure.story("Login by user/password");
    allure.parameter("browser", "firefox");

    open_login_page();
}

#[step]
fn open_login_page() {
    // your step implementation
}
```

The `#[allure_test]` attribute initializes a reporter automatically (using
`ALLURE_RESULTS_DIR` or `target/allure-results`) and wraps the test lifecycle.

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
  - global `allure()` accessor backed by `OnceLock`.

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
framework-specific helpers similar to `#[allure_test]` from `allure-cargotest`.

## Why is there a separate `allure-test-macros` crate?

Rust requires attribute procedural macros to be compiled from a `proc-macro` crate.
A `proc-macro` crate cannot serve as a regular runtime library crate at the same
time, so runtime/reporter APIs stay in `allure-cargotest` and macro implementation
lives in `allure-test-macros`.

From a user perspective this remains a single entrypoint because
`allure-cargotest` re-exports `#[allure_test]` and `#[step]`.


## Community

- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Contributing Guide](CONTRIBUTING.md)
- [Security Policy](SECURITY.md)
