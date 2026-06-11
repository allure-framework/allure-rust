use allure_cargotest::allure_test;
use allure_rust_commons::{attachment, feature, log_step, parameter, stage};

#[allure_test]
#[test]
fn uses_commons_runtime_functions() {
    feature("Runtime functions");
    parameter("source", "commons");
    stage("log from commons");
    log_step("logged from commons");
    stage("attach from commons");
    attachment("commons.txt", "text/plain", "attached from commons");
}
