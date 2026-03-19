pub mod error_classifier;
pub mod facade;
pub mod lifecycle;
pub(crate) mod md5;
pub mod model;
pub mod writer;

pub use facade::{allure, AllureFacade, StepGuard};
pub use lifecycle::{AllureLifecycle, AllureRuntime, StartTestCaseParams};
pub use md5::md5_hex;
pub use model::*;
pub use writer::FileSystemResultsWriter;
