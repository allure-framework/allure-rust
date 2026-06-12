use std::panic::Location;

use crate::{test_with, TestOptions};

#[track_caller]
pub(crate) fn allure_test<F>(module_path: &str, test_name: &str, description: &str, body: F)
where
    F: FnOnce(),
{
    test_with(
        TestOptions::new(test_name)
            .with_full_name(format!("{module_path}::{test_name}"))
            .with_description(description)
            .with_source(
                Location::caller().file(),
                env!("CARGO_MANIFEST_DIR"),
                module_path,
            ),
        body,
    );
}
