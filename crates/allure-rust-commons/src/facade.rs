use std::{
    panic::{self, AssertUnwindSafe},
    sync::OnceLock,
};

use crate::{
    error_classifier,
    lifecycle::{AllureLifecycle, StartTestCaseParams},
    model::{Status, StatusDetails},
};

static ALLURE: OnceLock<AllureFacade> = OnceLock::new();

pub fn allure() -> &'static AllureFacade {
    ALLURE.get_or_init(AllureFacade::default)
}

#[derive(Clone, Default)]
pub struct AllureFacade {
    lifecycle: Option<AllureLifecycle>,
}

impl AllureFacade {
    pub fn with_lifecycle(lifecycle: AllureLifecycle) -> Self {
        Self {
            lifecycle: Some(lifecycle),
        }
    }

    pub fn set_lifecycle(&mut self, lifecycle: AllureLifecycle) {
        self.lifecycle = Some(lifecycle);
    }

    pub fn start_test_case(&self, params: impl Into<StartTestCaseParams>) {
        if let Some(l) = &self.lifecycle {
            l.start_test_case(params);
        }
    }

    pub fn stop_test_case(&self, status: Status, details: Option<StatusDetails>) {
        if let Some(l) = &self.lifecycle {
            l.stop_test_case(status, details);
        }
    }

    pub fn start_test(&self, name: impl Into<String>) {
        self.start_test_case(name.into());
    }

    pub fn start_test_with_full_name(&self, name: impl Into<String>, full_name: impl Into<String>) {
        self.start_test_case(StartTestCaseParams::new(name).with_full_name(full_name));
    }

    pub fn end_test(&self, status: Status, details: Option<StatusDetails>) {
        self.stop_test_case(status, details);
    }

    pub fn description(&self, description: impl Into<String>) {
        if let Some(l) = &self.lifecycle {
            l.update_test_case(|t| t.description = Some(description.into()));
        }
    }

    pub fn description_html(&self, description: impl Into<String>) {
        if let Some(l) = &self.lifecycle {
            l.update_test_case(|t| t.description_html = Some(description.into()));
        }
    }

    pub fn label(&self, name: impl Into<String>, value: impl Into<String>) {
        if let Some(l) = &self.lifecycle {
            l.add_label(name, value);
        }
    }

    pub fn labels<I, K, V>(&self, labels: I)
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (k, v) in labels {
            self.label(k, v);
        }
    }

    pub fn link(&self, url: impl Into<String>, name: Option<String>, link_type: Option<String>) {
        if let Some(l) = &self.lifecycle {
            l.add_link(url, name, link_type);
        }
    }

    pub fn links<I, U, N, T>(&self, links: I)
    where
        I: IntoIterator<Item = (U, Option<N>, Option<T>)>,
        U: Into<String>,
        N: Into<String>,
        T: Into<String>,
    {
        for (url, name, link_type) in links {
            self.link(url, name.map(Into::into), link_type.map(Into::into));
        }
    }

    pub fn parameter(&self, name: impl Into<String>, value: impl Into<String>) {
        if let Some(l) = &self.lifecycle {
            l.add_parameter(name, value);
        }
    }

    pub fn test_case_id(&self, value: impl Into<String>) {
        if let Some(l) = &self.lifecycle {
            l.set_test_case_id(value);
        }
    }

    pub fn attachment(
        &self,
        name: impl Into<String>,
        content_type: impl Into<String>,
        body: impl AsRef<[u8]>,
    ) {
        if let Some(l) = &self.lifecycle {
            l.add_attachment(name, content_type, body.as_ref());
        }
    }

    pub fn step(&self, name: impl Into<String>) -> StepGuard {
        if let Some(l) = &self.lifecycle {
            l.start_step(name);
            StepGuard {
                lifecycle: self.lifecycle.clone(),
                status: Some(Status::Passed),
                details: None,
            }
        } else {
            StepGuard {
                lifecycle: None,
                status: None,
                details: None,
            }
        }
    }

    pub fn step_with<T, F>(&self, name: impl Into<String>, body: F) -> T
    where
        F: FnOnce() -> T,
    {
        let guard = self.step(name);
        let outcome = panic::catch_unwind(AssertUnwindSafe(body));
        match outcome {
            Ok(value) => {
                drop(guard);
                value
            }
            Err(payload) => {
                let (status, details) = error_classifier::classify_panic(&payload);
                drop(guard.with_status(status, Some(details)));
                panic::resume_unwind(payload);
            }
        }
    }

    pub fn log_step(&self, name: impl Into<String>) {
        self.log_step_with(name, None, None::<String>);
    }

    pub fn log_step_with<E>(
        &self,
        name: impl Into<String>,
        status: Option<Status>,
        error: Option<E>,
    ) where
        E: ToString,
    {
        if let Some(l) = &self.lifecycle {
            let timestamp = l.start_step_at(name, None);
            let status = status.unwrap_or(Status::Passed);
            let details = error.map(|error| StatusDetails {
                message: Some(error.to_string()),
                trace: None,
                actual: None,
                expected: None,
            });
            l.stop_step_at(Some(timestamp), status, details);
        }
    }

    pub fn step_display_name(&self, name: impl Into<String>) {
        if let Some(l) = &self.lifecycle {
            l.set_current_step_display_name(name);
        }
    }

    pub fn step_parameter(&self, name: impl Into<String>, value: impl Into<String>) {
        if let Some(l) = &self.lifecycle {
            l.add_current_step_parameter(name, value);
        }
    }

    pub fn issue(&self, name: impl Into<String>, url: impl Into<String>) {
        self.link(url.into(), Some(name.into()), Some("issue".to_string()));
    }

    pub fn tms(&self, name: impl Into<String>, url: impl Into<String>) {
        self.link(url.into(), Some(name.into()), Some("tms".to_string()));
    }

    pub fn epic(&self, value: impl Into<String>) {
        self.label("epic", value);
    }
    pub fn feature(&self, value: impl Into<String>) {
        self.label("feature", value);
    }
    pub fn story(&self, value: impl Into<String>) {
        self.label("story", value);
    }
    pub fn suite(&self, value: impl Into<String>) {
        self.label("suite", value);
    }
    pub fn parent_suite(&self, value: impl Into<String>) {
        self.label("parentSuite", value);
    }
    pub fn sub_suite(&self, value: impl Into<String>) {
        self.label("subSuite", value);
    }
    pub fn owner(&self, value: impl Into<String>) {
        self.label("owner", value);
    }
    pub fn severity(&self, value: impl Into<String>) {
        self.label("severity", value);
    }
    pub fn layer(&self, value: impl Into<String>) {
        self.label("layer", value);
    }
    pub fn tag(&self, value: impl Into<String>) {
        self.label("tag", value);
    }
    pub fn tags<I, V>(&self, tags: I)
    where
        I: IntoIterator<Item = V>,
        V: Into<String>,
    {
        for tag in tags {
            self.tag(tag);
        }
    }
    pub fn id(&self, value: impl Into<String>) {
        self.label("ALLURE_ID", value);
    }
}

pub struct StepGuard {
    lifecycle: Option<AllureLifecycle>,
    status: Option<Status>,
    details: Option<StatusDetails>,
}

impl StepGuard {
    pub fn failed(mut self, message: impl Into<String>) -> Self {
        self.status = Some(Status::Failed);
        self.details = Some(StatusDetails {
            message: Some(message.into()),
            trace: None,
            actual: None,
            expected: None,
        });
        self
    }

    pub fn broken(mut self, message: impl Into<String>) -> Self {
        self.status = Some(Status::Broken);
        self.details = Some(StatusDetails {
            message: Some(message.into()),
            trace: None,
            actual: None,
            expected: None,
        });
        self
    }

    pub fn with_status(mut self, status: Status, details: Option<StatusDetails>) -> Self {
        self.status = Some(status);
        self.details = details;
        self
    }
}

impl Drop for StepGuard {
    fn drop(&mut self) {
        if let (Some(l), Some(status)) = (&self.lifecycle, self.status.clone()) {
            l.stop_step(status, self.details.clone());
        }
    }
}
