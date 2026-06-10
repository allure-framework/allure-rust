use allure_cargotest::allure_test;

#[allure_test]
#[test]
fn writes_attachment() {
    allure.attachment("hello.txt", "text/plain", "hello from attachments sample");
}
