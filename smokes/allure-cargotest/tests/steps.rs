use allure_cargotest::{allure_test, step};

#[step]
fn some_step_doing_something() {}

#[step(name = "Readable step title")]
fn some_step_with_custom_name() {}

#[allure_test]
#[test]
fn writes_steps() {
    {
        let _simple = allure.step("simple step");
    }

    allure.log_step("logged step");
    some_step_doing_something();
    some_step_with_custom_name();

    {
        let _failed = allure.step("failed step").failed("step failed");
    }

    {
        let broken = allure.step("broken parent").broken("step broken");
        {
            let _nested = allure.step("nested child");
            allure.attachment("nested.txt", "text/plain", "inside nested step");
        }
        drop(broken);
    }
}
