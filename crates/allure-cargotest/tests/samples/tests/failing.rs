use allure_cargotest::allure_test;

#[allure_test]
#[test]
fn still_writes_passed() {
    allure.label("suite", "failing-sample");
}

#[allure_test]
#[test]
fn fails_with_message() {
    panic!("expected failure from sample");
}
