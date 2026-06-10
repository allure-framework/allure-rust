pub mod config;
pub mod error_classifier;
pub mod facade;
pub mod http_exchange;
pub mod lifecycle;
pub(crate) mod md5;
pub mod model;
pub mod writer;

#[cfg(test)]
mod test_utils;

pub use config::{
    apply_common_runtime_labels, apply_config_labels, apply_synthetic_suite_labels,
    global_labels_from_environment, relative_file_path, title_path,
};
pub use facade::{allure, AllureFacade, StepGuard};
pub use http_exchange::*;
pub use lifecycle::{AllureLifecycle, AllureRuntime, StartTestCaseParams};
pub use md5::md5_hex;
pub use model::*;
pub use writer::{
    results_dir_from_env, FileSystemResultsWriter, ALLURE_RESULTS_DIR_ENV, DEFAULT_RESULTS_DIR,
};
