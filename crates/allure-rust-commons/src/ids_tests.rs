use std::collections::HashSet;
use std::thread;

use super::next_id;
use crate::test_utils::allure_test;

#[test]
fn concurrently_generated_ids_are_unique() {
    allure_test(
        module_path!(),
        "concurrently_generated_ids_are_unique",
        "Verifies parallel id generation never repeats a value within a process.",
        || {
            const THREADS: usize = 8;
            const PER_THREAD: usize = 2_000;

            let handles = (0..THREADS)
                .map(|_| thread::spawn(|| (0..PER_THREAD).map(|_| next_id()).collect::<Vec<_>>()))
                .collect::<Vec<_>>();

            let ids = handles
                .into_iter()
                .flat_map(|handle| handle.join().expect("id generation thread panicked"))
                .collect::<Vec<_>>();

            let unique = ids.iter().collect::<HashSet<_>>();
            assert_eq!(
                unique.len(),
                THREADS * PER_THREAD,
                "generated ids collided within a single process"
            );
        },
    );
}

#[test]
fn ids_carry_the_process_identity() {
    allure_test(
        module_path!(),
        "ids_carry_the_process_identity",
        "Verifies ids embed the pid so separate test processes cannot collide.",
        || {
            let id = next_id();
            let pid = std::process::id().to_string();

            assert!(
                id.split('-').any(|part| part == pid),
                "id {id:?} does not contain pid {pid}"
            );
        },
    );
}

#[test]
fn successive_ids_differ_within_the_same_millisecond() {
    allure_test(
        module_path!(),
        "successive_ids_differ_within_the_same_millisecond",
        "Verifies id uniqueness does not depend on the clock advancing by a millisecond.",
        || {
            let ids = (0..64).map(|_| next_id()).collect::<HashSet<_>>();

            assert_eq!(ids.len(), 64, "ids repeated inside one millisecond");
        },
    );
}
