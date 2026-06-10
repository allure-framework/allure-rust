use allure_cargotest::allure_test;

#[allure_test]
#[test]
#[should_panic]
fn should_panic_without_expected_passes() {
    panic!("boom");
}

#[allure_test]
#[test]
#[should_panic(expected = "boom")]
fn should_panic_with_expected_passes() {
    panic!("boom goes here");
}

#[allure_test]
#[test]
#[should_panic(expected = "needle")]
fn should_panic_with_expected_mismatch_fails() {
    panic!("different panic message");
}

#[allure_test]
#[test]
#[should_panic]
fn should_panic_without_panic_fails() {}
