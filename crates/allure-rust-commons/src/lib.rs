//! Core Allure runtime, lifecycle, model, writer, and facade APIs for Rust integrations.
//!
//! The high-level facade functions are framework-neutral and operate on the current thread-bound
//! Allure context when one is active. Lower-level modules expose lifecycle and serialization
//! primitives for adapter authors.

#![deny(missing_docs)]

/// Runtime configuration helpers.
pub mod config;
pub(crate) mod current_owner;
/// Panic and error status classification helpers.
pub mod error_classifier;
/// High-level runtime facade and thread-bound helper functions.
pub mod facade;
/// Allure HTTP Exchange attachment model.
pub mod http_exchange;
/// Low-level lifecycle owner.
pub mod lifecycle;
pub(crate) mod md5;
/// Serializable Allure result model.
pub mod model;
/// Exact-owner reporter helpers for adapter internals.
pub mod reporter;
/// Filesystem writer for Allure result artifacts.
pub mod writer;

#[cfg(test)]
mod test_utils;

pub use config::{
    apply_common_runtime_labels, apply_config_labels, apply_synthetic_suite_labels, global_config,
    global_labels_from_environment, log_asserts_enabled, relative_file_path, title_path,
    GlobalAllureConfig,
};
pub use facade::{
    allure, allure_id, attach_trace, attach_trace_named, attachment, attachment_path,
    clear_last_assertion_failure, current_allure, description, description_html, display_name,
    epic, fail_assertion, feature, global_attachment, global_attachment_path, global_error,
    global_error_with_trace, history_id, http_exchange, http_exchange_named, id, issue, label,
    labels, layer, link, links, log_step, log_step_with, owner, parameter, parameter_excluded,
    parameter_mode, parameter_with_options, parent_suite, push_current_allure,
    record_assertion_pass, set_title_path, severity, stage, status_details_for_message, step,
    story, sub_suite, suite, tag, tags, test, test_async, test_case_id, test_named,
    test_named_async, test_with, test_with_async, tms, AllureFacade, CurrentAllureGuard,
    TestOptions,
};
pub use http_exchange::*;
pub use lifecycle::{AllureLifecycle, AllureRuntime, StartTestCaseParams};
pub use md5::md5_hex;
pub use model::*;
pub use writer::{
    results_dir_from_env, FileSystemResultsWriter, ALLURE_RESULTS_DIR_ENV, DEFAULT_RESULTS_DIR,
    PLAYWRIGHT_TRACE_ATTACHMENT_EXTENSION, PLAYWRIGHT_TRACE_ATTACHMENT_MIME,
};
