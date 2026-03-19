use std::{env, sync::OnceLock};

use allure_rust_commons::AllureFacade;

static HOST_NAME: OnceLock<Option<String>> = OnceLock::new();

pub(crate) fn add_default_and_global_labels(allure: &AllureFacade) {
    allure.label("language", "rust");
    allure.label("framework", "cargo-test");

    if let Some(host) = detect_host_name() {
        allure.label("host", host);
    }

    allure.label("thread", detect_thread_name());

    for (name, value) in global_labels_from_environment() {
        allure.label(name, value);
    }
}

fn detect_host_name() -> Option<String> {
    if let Ok(host_name) = env::var("ALLURE_HOST_NAME") {
        let trimmed = host_name.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    HOST_NAME.get_or_init(resolve_host_name).clone()
}

fn resolve_host_name() -> Option<String> {
    #[cfg(unix)]
    {
        use std::os::raw::{c_char, c_int};

        unsafe extern "C" {
            fn gethostname(name: *mut c_char, len: usize) -> c_int;
        }

        let mut buf = [0_u8; 256];
        // SAFETY: `buf` is a valid writable buffer and its length is correctly provided.
        let result = unsafe { gethostname(buf.as_mut_ptr().cast(), buf.len()) };
        if result == 0 {
            let len = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
            let host_name = String::from_utf8_lossy(&buf[..len]).trim().to_string();
            if !host_name.is_empty() {
                return Some(host_name);
            }
        }
    }

    env::var("HOSTNAME")
        .ok()
        .or_else(|| env::var("COMPUTERNAME").ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn detect_thread_name() -> String {
    if let Ok(thread_name) = env::var("ALLURE_THREAD_NAME") {
        let trimmed = thread_name.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    std::thread::current()
        .name()
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("{:?}", std::thread::current().id()))
}

fn global_labels_from_environment() -> Vec<(String, String)> {
    let mut labels = Vec::new();

    for (key, value) in env::vars() {
        if let Some(name) = key.strip_prefix("ALLURE_LABEL_") {
            if !name.is_empty() && !value.is_empty() {
                labels.push((name.to_string(), value.clone()));
            }
        }

        if let Some(name) = key.strip_prefix("allure.label.") {
            if !name.is_empty() && !value.is_empty() {
                labels.push((name.to_string(), value));
            }
        }
    }

    labels
}

pub(crate) fn add_synthetic_suite_labels(allure: &AllureFacade, full_name: Option<&str>) {
    let Some(full_name) = full_name else {
        return;
    };

    let mut segments = full_name.split("::").collect::<Vec<_>>();
    if segments.len() < 2 {
        return;
    }

    segments.pop();
    match segments.as_slice() {
        [] => {}
        [suite] => allure.suite(*suite),
        [parent_suite, suite] => {
            allure.parent_suite(*parent_suite);
            allure.suite(*suite);
        }
        [parent_suite, suite, rest @ ..] => {
            allure.parent_suite(*parent_suite);
            allure.suite(*suite);
            allure.sub_suite(rest.join("::"));
        }
    }
}
