use allure_cargotest::{allure_test, step};
use allure_cargotest::Status;

#[step]
fn some_step_doing_something() {}

#[step(name = "Readable step title")]
fn some_step_with_custom_name() {}

#[allure_test]
#[test]
fn writes_steps() {
    allure.description("Verifies explicit steps, macro steps, status overrides, nested steps, and nested attachments.");
    allure.step("simple step", || {});

    allure.log_step("logged step");
    some_step_doing_something();
    some_step_with_custom_name();

    allure.log_step_with("failed step", Some(Status::Failed), Some("step failed"));
    allure.log_step_with("broken step", Some(Status::Broken), Some("step broken"));

    allure.step("nested parent", || {
        allure.step("nested child", || {
            allure.attachment("nested.txt", "text/plain", "inside nested step");
        });
    });
}
