use allure_cargotest::allure_test;

#[allure_test]
#[test]
fn writes_attachment() {
    allure.description("Verifies inline text attachments are written with their configured name and media type.");
    allure.log_step("record text attachment");
    allure.attachment("hello.txt", "text/plain", "hello from attachments sample");
}
