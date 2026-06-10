use std::{
    collections::HashMap,
    env, fs,
    path::Path,
    sync::{Mutex, OnceLock},
};

use allure_rust_commons::AllureFacade;

static HOST_NAME: OnceLock<Option<String>> = OnceLock::new();
static CONFIGS: OnceLock<Mutex<HashMap<String, AllureConfig>>> = OnceLock::new();

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

pub(crate) fn add_config_labels(
    allure: &AllureFacade,
    manifest_dir: &str,
    module_path: &str,
    title_path: &[String],
) {
    let config = config_for(manifest_dir);
    let title_path = title_path.join("/");

    for (name, value) in &config.labels {
        allure.label(name, value);
    }

    for module in &config.modules {
        if module.matches(module_path, &title_path) {
            for (name, value) in &module.labels {
                allure.label(name, value);
            }
        }
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

fn config_for(manifest_dir: &str) -> AllureConfig {
    let manifest_dir = normalize_path(manifest_dir);
    let cache = CONFIGS.get_or_init(|| Mutex::new(HashMap::new()));

    if let Some(config) = cache
        .lock()
        .expect("poisoned allure config cache")
        .get(&manifest_dir)
        .cloned()
    {
        return config;
    }

    let config = read_config(&manifest_dir);
    cache
        .lock()
        .expect("poisoned allure config cache")
        .entry(manifest_dir)
        .or_insert_with(|| config.clone())
        .clone()
}

fn read_config(manifest_dir: &str) -> AllureConfig {
    let manifest = Path::new(manifest_dir).join("Cargo.toml");
    fs::read_to_string(manifest)
        .map(|source| parse_config(&source))
        .unwrap_or_default()
}

#[derive(Clone, Debug, Default)]
struct AllureConfig {
    labels: Vec<(String, String)>,
    modules: Vec<ModuleConfig>,
}

#[derive(Clone, Debug, Default)]
struct ModuleConfig {
    path: Option<String>,
    module: Option<String>,
    title_path: Option<Vec<String>>,
    labels: Vec<(String, String)>,
}

impl ModuleConfig {
    fn matches(&self, module_path: &str, title_path: &str) -> bool {
        let path_matches = self.path.as_ref().is_some_and(|path| {
            if path.ends_with('/') {
                title_path.starts_with(path)
            } else {
                title_path == path
            }
        });

        let title_path_matches = self
            .title_path
            .as_ref()
            .is_some_and(|expected| expected.join("/") == title_path);

        let module_matches = self.module.as_ref().is_some_and(|module| {
            module_path == module || module_path.starts_with(&format!("{module}::"))
        });

        path_matches || title_path_matches || module_matches
    }
}

#[derive(Clone, Copy)]
enum ConfigSection {
    Ignore,
    Root,
    Labels,
    Module(usize),
    ModuleLabels(usize),
}

fn parse_config(source: &str) -> AllureConfig {
    let mut config = AllureConfig::default();
    let mut section = ConfigSection::Ignore;

    for raw_line in source.lines() {
        let line = strip_comment(raw_line);
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(header) = table_header(line) {
            section = match header.as_str() {
                "[package.metadata.allure]" => ConfigSection::Root,
                "[package.metadata.allure.labels]" => ConfigSection::Labels,
                "[[package.metadata.allure.modules]]" => {
                    config.modules.push(ModuleConfig::default());
                    ConfigSection::Module(config.modules.len() - 1)
                }
                "[package.metadata.allure.modules.labels]" => {
                    if config.modules.is_empty() {
                        ConfigSection::Ignore
                    } else {
                        ConfigSection::ModuleLabels(config.modules.len() - 1)
                    }
                }
                _ => ConfigSection::Ignore,
            };
            continue;
        }

        let Some((key, value)) = split_key_value(line) else {
            continue;
        };
        let key = normalize_key(key);

        match section {
            ConfigSection::Root if key == "labels" => {
                config.labels.extend(parse_inline_labels(value));
            }
            ConfigSection::Labels => {
                config.labels.extend(parse_label_values(key, value));
            }
            ConfigSection::Module(index) => {
                let Some(module) = config.modules.get_mut(index) else {
                    continue;
                };
                match key.as_str() {
                    "path" => module.path = parse_string(value).map(|value| normalize_path(&value)),
                    "module" => module.module = parse_string(value),
                    "title_path" => module.title_path = parse_string_array(value),
                    "labels" => module.labels.extend(parse_inline_labels(value)),
                    _ => {}
                }
            }
            ConfigSection::ModuleLabels(index) => {
                if let Some(module) = config.modules.get_mut(index) {
                    module.labels.extend(parse_label_values(key, value));
                }
            }
            ConfigSection::Ignore | ConfigSection::Root => {}
        }
    }

    config
}

fn table_header(line: &str) -> Option<String> {
    let line = line.trim();
    if line.starts_with("[[") && line.ends_with("]]") {
        return Some(line.to_string());
    }
    if line.starts_with('[') && line.ends_with(']') {
        return Some(line.to_string());
    }
    None
}

fn strip_comment(line: &str) -> &str {
    let mut quote = None;
    let mut escaped = false;

    for (idx, ch) in line.char_indices() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
                continue;
            }
            if active_quote == '"' && ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == active_quote {
                quote = None;
            }
            continue;
        }

        match ch {
            '"' | '\'' => quote = Some(ch),
            '#' => return &line[..idx],
            _ => {}
        }
    }

    line
}

fn split_key_value(input: &str) -> Option<(&str, &str)> {
    let mut quote = None;
    let mut escaped = false;

    for (idx, ch) in input.char_indices() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
                continue;
            }
            if active_quote == '"' && ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == active_quote {
                quote = None;
            }
            continue;
        }

        match ch {
            '"' | '\'' => quote = Some(ch),
            '=' => return Some((&input[..idx], &input[idx + 1..])),
            _ => {}
        }
    }

    None
}

fn parse_inline_labels(value: &str) -> Vec<(String, String)> {
    let value = value.trim();
    let Some(value) = value
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
    else {
        return Vec::new();
    };

    split_top_level(value, ',')
        .into_iter()
        .flat_map(|entry| {
            let Some((key, value)) = split_key_value(entry) else {
                return Vec::new();
            };
            parse_label_values(normalize_key(key), value)
        })
        .collect()
}

fn parse_label_values(key: String, value: &str) -> Vec<(String, String)> {
    if let Some(value) = parse_string(value) {
        return vec![(key, value)];
    }

    parse_string_array(value)
        .unwrap_or_default()
        .into_iter()
        .map(|value| (key.clone(), value))
        .collect()
}

fn parse_string_array(value: &str) -> Option<Vec<String>> {
    let value = value.trim();
    let value = value.strip_prefix('[')?.strip_suffix(']')?;
    let values = split_top_level(value, ',')
        .into_iter()
        .filter_map(parse_string)
        .collect::<Vec<_>>();

    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

fn split_top_level(input: &str, delimiter: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut quote = None;
    let mut escaped = false;
    let mut depth = 0_u32;

    for (idx, ch) in input.char_indices() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
                continue;
            }
            if active_quote == '"' && ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == active_quote {
                quote = None;
            }
            continue;
        }

        match ch {
            '"' | '\'' => quote = Some(ch),
            '[' | '{' | '(' => depth = depth.saturating_add(1),
            ']' | '}' | ')' => depth = depth.saturating_sub(1),
            _ if ch == delimiter && depth == 0 => {
                parts.push(input[start..idx].trim());
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }

    parts.push(input[start..].trim());
    parts
}

fn parse_string(value: &str) -> Option<String> {
    let value = value.trim();
    if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
        return Some(unescape_basic_string(&value[1..value.len() - 1]));
    }
    if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2 {
        return Some(value[1..value.len() - 1].to_string());
    }
    None
}

fn unescape_basic_string(value: &str) -> String {
    let mut result = String::new();
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }

        match chars.next() {
            Some('n') => result.push('\n'),
            Some('r') => result.push('\r'),
            Some('t') => result.push('\t'),
            Some('"') => result.push('"'),
            Some('\\') => result.push('\\'),
            Some(other) => {
                result.push('\\');
                result.push(other);
            }
            None => result.push('\\'),
        }
    }

    result
}

fn normalize_key(key: &str) -> String {
    let key = key.trim();
    parse_string(key).unwrap_or_else(|| key.to_string())
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
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

#[cfg(test)]
mod tests {
    use crate::test_utils::allure_test;

    use super::parse_config;

    #[test]
    fn parses_package_metadata_allure_labels() {
        allure_test(
            module_path!(),
            "parses_package_metadata_allure_labels",
            || {
                let config = parse_config(
                    r#"
[package.metadata.allure.labels]
module = "checkout"
layer = "e2e"
tag = ["smoke", "regression"]

[[package.metadata.allure.modules]]
path = "tests/payments.rs"
labels = { component = "payments", owner = "qa", tag = ["payments-smoke", "payments-regression"] }

[[package.metadata.allure.modules]]
module = "payments::cards"
[package.metadata.allure.modules.labels]
feature = "cards"
story = ["visa", "mastercard"]
"#,
                );

                assert_eq!(
                    config.labels,
                    vec![
                        ("module".to_string(), "checkout".to_string()),
                        ("layer".to_string(), "e2e".to_string()),
                        ("tag".to_string(), "smoke".to_string()),
                        ("tag".to_string(), "regression".to_string()),
                    ]
                );
                assert!(config.modules[0].matches("payments", "tests/payments.rs"));
                assert!(config.modules[1].matches("payments::cards::visa", "src/cards.rs"));
                assert_eq!(
                    config.modules[0].labels,
                    vec![
                        ("component".to_string(), "payments".to_string()),
                        ("owner".to_string(), "qa".to_string()),
                        ("tag".to_string(), "payments-smoke".to_string()),
                        ("tag".to_string(), "payments-regression".to_string()),
                    ]
                );
                assert_eq!(
                    config.modules[1].labels,
                    vec![
                        ("feature".to_string(), "cards".to_string()),
                        ("story".to_string(), "visa".to_string()),
                        ("story".to_string(), "mastercard".to_string()),
                    ]
                );
            },
        );
    }
}
