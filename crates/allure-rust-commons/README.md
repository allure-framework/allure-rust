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
    allure.log_step("open login page");
    allure.attachment("request.json", "application/json", br#"{"ok":true}"#);
    allure.end_test(Status::Passed, None);

    Ok(())
}
```

## Common building blocks

- `AllureRuntime`: owns the configured results writer.
- `AllureLifecycle`: manages in-progress tests and steps.
- `AllureFacade`: ergonomic helper for labels, links, parameters, steps, and attachments.
- `FileSystemResultsWriter`: persists JSON result files and attachments.
- `model`: raw Allure data structures for custom integrations.

## Output location

The writer creates the target directory if it does not already exist:

```rust
let writer = FileSystemResultsWriter::new("target/allure-results")?;
```
