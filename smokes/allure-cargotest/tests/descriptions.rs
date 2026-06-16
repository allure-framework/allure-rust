use allure_cargotest::allure_test;

#[allure_test]
#[test]
fn writes_descriptions() {
    allure.description("markdown description");
    allure.description_html("<p>html description</p>");
    allure.log_step("markdown and HTML descriptions were recorded");
}

/// doc comment description
#[allure_test]
#[test]
fn uses_doc_comment_as_description() {
    allure.log_step("doc comment description was applied by the macro");
}

/// skipped doc comment description
#[allure_test(doc = false)]
#[test]
fn skips_doc_comment_description() {
    allure.log_step("doc comment description was disabled by the macro argument");
}
