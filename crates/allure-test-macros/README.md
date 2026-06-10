# allure-test-macros

`allure-test-macros` contains the procedural macros used by the workspace.
In most cases you should depend on `allure-cargotest` instead, because it re-exports these
macros together with the runtime integration.

## Recommended usage

Add the higher-level crate:

```bash
cargo add allure-cargotest --dev
```

Then use the re-exported macros:

```rust
use allure_cargotest::{allure_test, step};

#[step(name = "Open login page")]
fn open_login_page() {}

#[allure_test(name = "Login works", id = "AUTH-1")]
#[test]
fn login_works() {
    open_login_page();
}
```

## What the macros do

- `#[allure_test]` wraps a `#[test]` function with Allure lifecycle setup and teardown.
- `#[step]` records a function call as an Allure step.
- Optional macro arguments let you override the displayed test or step name.

## Notes

- `#[allure_test]` supports synchronous `#[test]` functions and root async tests that compose with
  runtime-specific attributes such as `#[tokio::test]`.
- For Tokio tests, add Tokio to your test crate and place `#[allure_test]` above `#[tokio::test]`.
- Allure context is available in the root async test body and awaited helpers, but independently
  spawned tasks do not implicitly inherit it.
- Test results are written through `allure-rust-commons`, which honors `ALLURE_RESULTS_DIR`
  when set and otherwise uses `target/allure-results`.
- This crate is intended as an implementation crate for proc macros.
