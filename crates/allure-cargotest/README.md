# allure-cargotest

`allure-cargotest` is the main user-facing crate in this workspace.
It wires Allure test lifecycle handling into `cargo test`, writes result files, and
re-exports the `#[allure_test]` and `#[step]` macros.

## Add the crate

```bash
cargo add allure-cargotest --dev
```

## Basic usage

Annotate your tests with `#[allure_test]` and optional helper functions with `#[step]`.
During execution, results are written to `target/allure-results` by default.

```rust
use allure_cargotest::{allure_test, step};

#[step]
fn open_login_page() {
    // your step implementation
}

#[allure_test]
#[test]
fn login_works() {
    allure.epic("Web interface");
    allure.feature("Authentication");
    allure.story("Login with username and password");
    allure.parameter("browser", "firefox");

    open_login_page();
    allure.attachment("page.html", "text/html", "<html>...</html>");
}
```

## Tokio async tests

For Tokio tests, depend on Tokio in your test crate and place `#[allure_test]` above
`#[tokio::test]`. `allure-cargotest` does not depend on Tokio.

```rust
use allure_cargotest::allure_test;

#[allure_test]
#[tokio::test]
async fn login_works_async() {
    allure.feature("Authentication");
    tokio::task::yield_now().await;
    allure.parameter("runtime", "tokio");
}
```

The root async test body and awaited helpers can use Allure metadata and steps. Independently
spawned Tokio tasks do not implicitly inherit the current Allure context.

## Configure the output directory

Override the default results directory with `ALLURE_RESULTS_DIR`:

```bash
ALLURE_RESULTS_DIR=./allure-results cargo test
```

## Configure labels in Cargo.toml

Use Cargo package metadata to add labels for local and CI runs without environment variables:

### Add labels for all tests in a package

Add labels under `[package.metadata.allure.labels]` in the package `Cargo.toml`.
A package is the Cargo package defined by that `Cargo.toml`.

```toml
[package.metadata.allure.labels]
a = "a-value"
b = ["b-value1", "b-value2"]
```

This adds `a=a-value`, `b=b-value1`, and `b=b-value2` to every `#[allure_test]` in the package.
String array values add the same label multiple times.

### Add labels for only some Rust modules

Add one `[[package.metadata.allure.modules]]` entry per Rust module path.
The `module` value matches the current Rust `module_path!()` exactly, or any module below it.

```toml
[[package.metadata.allure.modules]]
module = "org::example"
labels = { a = "a-value", b = ["b-value1", "b-value2"] }
```

This applies to tests whose module path is `org::example`, `org::example::api`,
`org::example::api::v1`, and so on.

For integration tests in `tests/api.rs`, the test file is its own crate, so module paths usually
start with the file stem:

```toml
[[package.metadata.allure.modules]]
module = "api::org::example"
labels = { a = "a-value" }
```

### Add labels for only test files

Add one `[[package.metadata.allure.modules]]` entry per file and use `path`. The path is relative
to the package root and uses the same element-wise file path that appears in `titlePath`.

```toml
[[package.metadata.allure.modules]]
path = "tests/payments.rs"
labels = { a = "a-value", b = ["b-value1", "b-value2"] }
```

This applies to every `#[allure_test]` in `tests/payments.rs`.

You can also match a source file or a directory:

```toml
[[package.metadata.allure.modules]]
path = "src/payments.rs"
labels = { component = "payments" }

[[package.metadata.allure.modules]]
path = "tests/api/"
labels = { layer = "api" }
```

The directory form matches every test file whose relative path starts with that directory.

## Generate an Allure report

After the test run, generate and open a report with the Allure CLI:

```bash
allure generate ./target/allure-results --output ./target/allure-report --clean
allure open ./target/allure-report
```

## What this crate provides

- `CargoTestReporter` for manual integration when macros are not enough.
- Re-exported `#[allure_test]` and `#[step]` macros.
- Re-exported `Status` and `StatusDetails` types.
- Automatic lifecycle setup for `cargo test`.
