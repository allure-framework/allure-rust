//! Filesystem writer for Allure result artifacts.

use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

use crate::{
    config::global_config,
    http_exchange::{HTTP_EXCHANGE_ATTACHMENT_EXTENSION, HTTP_EXCHANGE_ATTACHMENT_MIME},
    model::{Categories, Globals, TestResult, TestResultContainer},
};

/// Environment variable used to override the Allure results directory.
pub const ALLURE_RESULTS_DIR_ENV: &str = "ALLURE_RESULTS_DIR";
/// Default Allure results directory.
pub const DEFAULT_RESULTS_DIR: &str = "target/allure-results";
/// MIME type used for Playwright trace attachments.
pub const PLAYWRIGHT_TRACE_ATTACHMENT_MIME: &str = "application/vnd.allure.playwright-trace";
/// File extension used for Playwright trace attachments.
pub const PLAYWRIGHT_TRACE_ATTACHMENT_EXTENSION: &str = ".zip";

/// Writes Allure result files and attachments atomically into a directory.
///
/// Each artifact is staged beside its target with a `.tmp` suffix, synced when
/// the filesystem supports it, and then published with an atomic rename.
#[derive(Debug, Clone)]
pub struct FileSystemResultsWriter {
    out_dir: PathBuf,
}

impl FileSystemResultsWriter {
    /// Creates a writer using the configured results directory.
    pub fn from_env() -> std::io::Result<Self> {
        Self::new(results_dir_from_env())
    }

    /// Creates a writer for the given output directory.
    pub fn new<P: AsRef<Path>>(out_dir: P) -> std::io::Result<Self> {
        fs::create_dir_all(&out_dir)?;
        Ok(Self {
            out_dir: out_dir.as_ref().to_path_buf(),
        })
    }

    /// Writes a test result JSON file.
    pub fn write_result(&self, result: &TestResult) -> std::io::Result<PathBuf> {
        self.write_result_typed(result)
    }

    /// Writes a typed test result JSON file.
    pub fn write_result_typed(&self, result: &TestResult) -> std::io::Result<PathBuf> {
        let path = self.out_dir.join(format!("{}-result.json", result.uuid));
        self.write_json(&path, result)?;
        Ok(path)
    }

    /// Writes a test result container JSON file.
    pub fn write_container(&self, container: &TestResultContainer) -> std::io::Result<PathBuf> {
        self.write_container_typed(container)
    }

    /// Writes a typed test result container JSON file.
    pub fn write_container_typed(
        &self,
        container: &TestResultContainer,
    ) -> std::io::Result<PathBuf> {
        let path = self
            .out_dir
            .join(format!("{}-container.json", container.uuid));
        self.write_json(&path, container)?;
        Ok(path)
    }

    /// Writes a globals JSON file for run-level diagnostics.
    pub fn write_globals(&self, globals: &Globals) -> std::io::Result<PathBuf> {
        self.write_globals_typed(globals)
    }

    /// Writes a typed globals JSON file for run-level diagnostics.
    pub fn write_globals_typed(&self, globals: &Globals) -> std::io::Result<PathBuf> {
        let path = self
            .out_dir
            .join(format!("{}-globals.json", uuid_like_name()));
        self.write_json(&path, globals)?;
        Ok(path)
    }

    /// Writes `environment.properties` with deterministic key ordering.
    pub fn write_environment_properties(
        &self,
        properties: &HashMap<String, String>,
    ) -> std::io::Result<PathBuf> {
        let path = self.out_dir.join("environment.properties");
        let mut keys = properties.keys().collect::<Vec<_>>();
        keys.sort_unstable();
        let content = keys
            .into_iter()
            .map(|k| format!("{}={}", k, properties[k]))
            .collect::<Vec<_>>()
            .join("\n");
        write_atomically(&path, content.as_bytes())?;
        Ok(path)
    }

    /// Writes the Allure categories file.
    pub fn write_categories(&self, categories: &Categories) -> std::io::Result<PathBuf> {
        self.write_categories_typed(categories)
    }

    /// Writes the typed Allure categories file.
    pub fn write_categories_typed(&self, categories: &Categories) -> std::io::Result<PathBuf> {
        let path = self.out_dir.join("categories.json");
        self.write_json(&path, categories)?;
        Ok(path)
    }

    /// Writes an attachment with an explicit source filename.
    pub fn write_attachment(&self, source_name: &str, bytes: &[u8]) -> std::io::Result<PathBuf> {
        self.write_attachment_named(source_name, bytes)
    }

    /// Writes an attachment with an explicit source filename.
    pub fn write_attachment_named(
        &self,
        source_name: &str,
        bytes: &[u8],
    ) -> std::io::Result<PathBuf> {
        let path = self.out_dir.join(source_name);
        write_atomically(&path, bytes)?;
        Ok(path)
    }

    /// Writes an attachment and returns the generated source filename and path.
    pub fn write_attachment_auto(
        &self,
        uuid: &str,
        attachment_name: Option<&str>,
        content_type: Option<&str>,
        bytes: &[u8],
    ) -> std::io::Result<(String, PathBuf)> {
        let source_name = attachment_source_name(uuid, attachment_name, content_type);
        let path = self.out_dir.join(&source_name);
        write_atomically(&path, bytes)?;
        Ok((source_name, path))
    }

    fn write_json<T: serde::Serialize>(&self, path: &Path, value: &T) -> std::io::Result<()> {
        let json = serde_json::to_vec(value)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        write_atomically(path, &json)
    }
}

fn write_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temporary_path = temporary_path(path);
    let mut temporary_file = File::create(&temporary_path)?;

    let write_result = (|| {
        temporary_file.write_all(bytes)?;
        sync_if_supported(&temporary_file)
    })();
    drop(temporary_file);

    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }

    if let Err(error) = fs::rename(&temporary_path, path) {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }

    Ok(())
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut temporary_path = path.as_os_str().to_os_string();
    temporary_path.push(".tmp");
    PathBuf::from(temporary_path)
}

fn sync_if_supported(file: &File) -> io::Result<()> {
    match file.sync_all() {
        Err(error) if error.kind() == io::ErrorKind::Unsupported => Ok(()),
        result => result,
    }
}

/// Returns the configured Allure results directory.
pub fn results_dir_from_env() -> PathBuf {
    global_config().results_dir().to_path_buf()
}

pub(crate) fn attachment_source_name(
    uuid: &str,
    attachment_name: Option<&str>,
    content_type: Option<&str>,
) -> String {
    let ext = resolve_attachment_extension(attachment_name, content_type);
    format!("{}-attachment{}", uuid, ext)
}

fn resolve_attachment_extension(
    attachment_name: Option<&str>,
    content_type: Option<&str>,
) -> String {
    if let Some(ext) = extension_from_name(attachment_name) {
        return ext;
    }
    if let Some(ext) = extension_from_content_type(content_type) {
        return ext;
    }
    String::new()
}

fn extension_from_name(name: Option<&str>) -> Option<String> {
    let name = name?;
    let ext = Path::new(name).extension().and_then(OsStr::to_str)?;
    if ext.is_empty() {
        None
    } else {
        Some(format!(".{ext}"))
    }
}

fn extension_from_content_type(content_type: Option<&str>) -> Option<String> {
    let ct = content_type?.split(';').next()?.trim();
    let ext = match ct {
        "text/plain" => ".txt",
        "text/html" => ".html",
        "text/csv" => ".csv",
        "text/xml" => ".xml",
        "application/json" => ".json",
        HTTP_EXCHANGE_ATTACHMENT_MIME => HTTP_EXCHANGE_ATTACHMENT_EXTENSION,
        "application/xml" => ".xml",
        "application/yaml" | "application/x-yaml" | "text/yaml" => ".yaml",
        "image/png" => ".png",
        "image/jpeg" => ".jpg",
        "image/gif" => ".gif",
        "image/svg+xml" => ".svg",
        "video/mp4" => ".mp4",
        PLAYWRIGHT_TRACE_ATTACHMENT_MIME => PLAYWRIGHT_TRACE_ATTACHMENT_EXTENSION,
        _ => return None,
    };
    Some(ext.to_string())
}

fn uuid_like_name() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static COUNTER: AtomicU64 = AtomicU64::new(1);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default();
    format!("{}-{}", now, COUNTER.fetch_add(1, Ordering::Relaxed))
}

#[cfg(test)]
#[path = "writer_tests.rs"]
mod writer_tests;
