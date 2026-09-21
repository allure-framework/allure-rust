//! Identifier generation for Allure artifacts.

use std::{
    collections::hash_map::RandomState,
    hash::{BuildHasher, Hasher},
    sync::{
        atomic::{AtomicU64, Ordering},
        OnceLock,
    },
    time::{SystemTime, UNIX_EPOCH},
};

static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Returns entropy that is stable within a process and differs between processes.
///
/// The counter alone only separates ids inside one process, so runners that spawn a
/// process per test would otherwise rely on the clock alone. The pid closes most of
/// that gap; this salt additionally covers pid reuse across containers or CI shards
/// writing into a shared results directory.
fn process_salt() -> u64 {
    static SALT: OnceLock<u64> = OnceLock::new();
    *SALT.get_or_init(|| RandomState::new().build_hasher().finish())
}

/// Returns a unique identifier for a test, container, or attachment artifact.
///
/// The timestamp comes first so generated file names keep a roughly chronological order.
pub(crate) fn next_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();

    format!(
        "{}-{}-{:x}-{}",
        nanos,
        std::process::id(),
        process_salt(),
        ID_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

#[cfg(test)]
#[path = "ids_tests.rs"]
mod ids_tests;
