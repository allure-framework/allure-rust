use allure_cargotest::allure_test;

#[allure_test]
#[test]
fn writes_descriptions() {
    allure.description("markdown description");
    allure.description_html("<p>html description</p>");
    allure.log_step("markdown and HTML descriptions were recorded");
}
