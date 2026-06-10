use allure_cargotest::allure_test;

#[allure_test]
#[test]
fn selected_by_name() {}

#[allure_test(id = "A-2")]
#[test]
fn selected_by_id() {}

#[allure_test]
#[test]
fn selected_extra() {}
