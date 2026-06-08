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
