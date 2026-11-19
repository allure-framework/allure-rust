use allure_cargotest::{allure_test, CargoTestReporter};
use std::process::{ExitCode, Termination};

fn forbidden() -> ! {
    std::fs::write("body-executed", "muted body ran").expect("write sentinel");
    panic!("muted body must not execute")
}

struct FailingTermination;
impl Termination for FailingTermination {
    fn report(self) -> ExitCode {
        std::fs::write("termination-reported", "report ran").expect("write sentinel");
        ExitCode::FAILURE
    }
}

/// Static metadata must survive muted finalization.
#[allure_test(name = "muted display name", id = "42")]
#[test]
fn sync_unit() {
    forbidden();
}

#[allure_test]
#[tokio::test]
async fn async_unit() {
    forbidden();
}

#[allure_test]
#[test]
#[should_panic(expected = "expected sentinel")]
fn sync_panic() {
    forbidden();
}

#[allure_test]
#[tokio::test]
#[should_panic(expected = "expected sentinel")]
async fn async_panic() {
    forbidden();
}

#[allure_test]
#[test]
fn sync_result() -> Result<(), &'static str> {
    forbidden();
}

#[allure_test]
#[tokio::test]
async fn async_result() -> Result<(), &'static str> {
    forbidden();
}

#[allure_test]
#[test]
fn sync_termination() -> FailingTermination {
    forbidden();
}

#[allure_test]
#[tokio::test]
async fn async_termination() -> FailingTermination {
    forbidden();
}

#[allure_test]
#[test]
fn sync_opaque() -> impl Termination {
    forbidden();
    #[allow(unreachable_code)]
    FailingTermination
}

#[allure_test]
#[tokio::test]
async fn async_opaque() -> impl Termination {
    forbidden();
    #[allow(unreachable_code)]
    FailingTermination
}

#[test]
fn manual_sync() {
    let reporter = CargoTestReporter::new(std::env::var("ALLURE_RESULTS_DIR").unwrap()).unwrap();
    reporter.run_test("manual_plain", |_| forbidden());
    reporter.run_test_with_metadata(
        "manual_metadata",
        Some("manual::metadata"),
        Some("51"),
        None,
        |_| forbidden(),
    );
    reporter.run_test_with_metadata("manual_tag", None, None, Some(&["@allure.id:52"]), |_| {
        forbidden()
    });
    reporter.run_test_with_result("manual_result", |_| forbidden());
}

#[tokio::test]
async fn manual_async() {
    let reporter = CargoTestReporter::new(std::env::var("ALLURE_RESULTS_DIR").unwrap()).unwrap();
    reporter
        .run_test_with_metadata_async("manual_async", Some("manual::async"), None, None, async {
            forbidden();
        })
        .await;
    reporter
        .run_test_with_result_async("manual_async_result", async { forbidden() })
        .await;
}

#[allure_test]
#[test]
fn control() {
    std::fs::write("control-executed", "control ran").expect("write control sentinel");
    assert_eq!(2 + 2, 4);
}

#[allure_test]
#[test]
#[ignore]
fn user_ignored() {
    forbidden();
}
