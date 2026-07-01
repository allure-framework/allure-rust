//! Diesel integration that records executed queries as Allure steps.
//!
//! This crate plugs into Diesel's [`Instrumentation`] hook, so it is wired up once in test or
//! setup code and never touches production query call sites:
//!
//! ```no_run
//! use diesel::prelude::*;
//! use diesel::sqlite::SqliteConnection;
//!
//! # fn demo() -> ConnectionResult<()> {
//! let mut conn = SqliteConnection::establish(":memory:")?;
//! conn.set_instrumentation(allure_diesel::AllureInstrumentation::new());
//! // From here on, every query executed on `conn` is recorded as an Allure step.
//! # Ok(())
//! # }
//! ```
//!
//! Alternatively, [`install_default`] registers the instrumentation for *every* newly
//! established connection process-wide.
//!
//! ## What is captured
//!
//! Each executed statement becomes a step named with its SQL, and (by default) carries the full
//! SQL as a `query.sql` attachment. Transactions become `transaction` steps that nest their
//! inner queries; a query that returns an error marks its step failed with the Diesel error.
//!
//! ## What is not captured
//!
//! Diesel's instrumentation exposes the SQL of each statement but **not the returned rows** —
//! no non-invasive Diesel hook does. This crate therefore records queries and transactions, not
//! result sets. The SQL text may contain sensitive bind values; keep that in mind before
//! publishing reports.

#![deny(missing_docs)]

use allure_rust_commons::{current_allure, StepGuard};
use diesel::connection::{set_default_instrumentation, Instrumentation, InstrumentationEvent};

const DEFAULT_MAX_SQL_PREVIEW: usize = 1024;

/// Controls what [`AllureInstrumentation`] records.
#[derive(Debug, Clone)]
pub struct CaptureOptions {
    capture_transactions: bool,
    capture_connection_events: bool,
    attach_sql: bool,
    max_sql_preview: usize,
}

impl CaptureOptions {
    /// Disables `transaction` steps for `BEGIN`/`COMMIT`/`ROLLBACK` boundaries.
    pub fn without_transactions(mut self) -> Self {
        self.capture_transactions = false;
        self
    }

    /// Enables steps for connection-establishment events (off by default).
    pub fn with_connection_events(mut self) -> Self {
        self.capture_connection_events = true;
        self
    }

    /// Disables the full-SQL `query.sql` attachment (the step name still shows the SQL).
    pub fn without_sql_attachment(mut self) -> Self {
        self.attach_sql = false;
        self
    }

    /// Sets the maximum number of characters used for the SQL preview in a step name.
    pub fn with_max_sql_preview(mut self, max_sql_preview: usize) -> Self {
        self.max_sql_preview = max_sql_preview;
        self
    }
}

impl Default for CaptureOptions {
    fn default() -> Self {
        Self {
            capture_transactions: true,
            capture_connection_events: false,
            attach_sql: true,
            max_sql_preview: DEFAULT_MAX_SQL_PREVIEW,
        }
    }
}

/// Diesel [`Instrumentation`] that records executed queries and transactions as Allure steps.
///
/// Attach it to a connection with `conn.set_instrumentation(AllureInstrumentation::new())`, or
/// register it globally with [`install_default`]. Steps are recorded against whichever Allure
/// facade is bound to the current thread when the query runs; outside a test context, events are
/// no-ops.
pub struct AllureInstrumentation {
    options: CaptureOptions,
    stack: Vec<Option<StepGuard>>,
    /// Set after `BeginTransaction` until the opening `BEGIN`/`SAVEPOINT` SQL finishes. If that
    /// boundary fails, the transaction never started and no commit/rollback will follow, so the
    /// transaction step is closed with the failure instead of leaking open.
    opening_transaction: bool,
    /// Set when a `CommitTransaction`/`RollbackTransaction` event has been seen but the boundary
    /// `COMMIT`/`ROLLBACK` SQL has not finished yet, so the transaction step stays open until it
    /// does (and inherits any boundary failure).
    closing_transaction: bool,
}

impl AllureInstrumentation {
    /// Creates instrumentation with the default [`CaptureOptions`].
    pub fn new() -> Self {
        Self::with_options(CaptureOptions::default())
    }

    /// Creates instrumentation with explicit [`CaptureOptions`].
    pub fn with_options(options: CaptureOptions) -> Self {
        Self {
            options,
            stack: Vec::new(),
            opening_transaction: false,
            closing_transaction: false,
        }
    }

    /// Opens a step against the current thread-bound facade, tracking its guard on the stack.
    ///
    /// When no facade is bound, a `None` placeholder is pushed so start/finish events stay
    /// balanced.
    fn push_step(&mut self, name: String, sql: Option<&str>) {
        let guard = current_allure().map(|allure| {
            let guard = allure.enter_step(name);
            if self.options.attach_sql {
                if let Some(sql) = sql {
                    // Nests a `query.sql` step holding the full SQL under the query step.
                    allure.attachment("query.sql", "text/sql", sql.as_bytes());
                }
            }
            guard
        });
        self.stack.push(guard);
    }

    /// Closes the most recently opened step, marking it failed when an error message is present.
    ///
    /// A `None` guard means the step was opened without a bound facade, and an empty stack means
    /// an unbalanced finish event; both are safe to ignore.
    fn pop_step(&mut self, error: Option<String>) {
        if let Some(Some(mut guard)) = self.stack.pop() {
            if let Some(message) = error {
                guard.fail(message);
            }
        }
    }
}

impl Default for AllureInstrumentation {
    fn default() -> Self {
        Self::new()
    }
}

impl Instrumentation for AllureInstrumentation {
    fn on_connection_event(&mut self, event: InstrumentationEvent<'_>) {
        match event {
            InstrumentationEvent::StartQuery { query, .. } => {
                let sql = query.to_string();
                let name = sql_preview(&sql, self.options.max_sql_preview);
                self.push_step(name, Some(&sql));
            }
            InstrumentationEvent::FinishQuery { error, .. } => {
                let error = error.map(ToString::to_string);
                self.pop_step(error.clone());
                if self.opening_transaction {
                    // The just-finished query was the BEGIN/SAVEPOINT opening boundary. On success
                    // the transaction step stays open for the inner queries; on failure the
                    // transaction never started and no COMMIT/ROLLBACK will follow, so close it now
                    // with the boundary error rather than leaking it open.
                    self.opening_transaction = false;
                    if error.is_some() {
                        self.pop_step(error);
                    }
                } else if self.closing_transaction {
                    // The just-finished query was the COMMIT/ROLLBACK boundary: close the
                    // transaction step that was held open for it, propagating a boundary failure so
                    // a failed COMMIT or ROLLBACK is not reported as a passed transaction.
                    self.closing_transaction = false;
                    self.pop_step(error);
                }
            }
            InstrumentationEvent::BeginTransaction { depth, .. }
                if self.options.capture_transactions =>
            {
                self.push_step(format!("transaction (depth {depth})"), None);
                // Diesel runs the BEGIN/SAVEPOINT SQL next; its FinishQuery decides whether the
                // transaction step stays open (success) or closes as failed (see FinishQuery).
                self.opening_transaction = true;
            }
            // Diesel emits these BEFORE running the actual COMMIT/ROLLBACK SQL. Keep the
            // transaction step open so the boundary query nests inside it and its outcome (via the
            // next FinishQuery) closes the step and sets its status.
            InstrumentationEvent::CommitTransaction { .. }
            | InstrumentationEvent::RollbackTransaction { .. }
                if self.options.capture_transactions =>
            {
                self.closing_transaction = true;
            }
            InstrumentationEvent::StartEstablishConnection { .. }
                if self.options.capture_connection_events =>
            {
                self.push_step("connect".to_string(), None);
            }
            InstrumentationEvent::FinishEstablishConnection { error, .. }
                if self.options.capture_connection_events =>
            {
                self.pop_step(error.map(ToString::to_string));
            }
            // `CacheQuery`, option-disabled events, and future non-exhaustive variants need no step.
            _ => {}
        }
    }
}

/// Registers [`AllureInstrumentation`] as the default instrumentation for every newly
/// established Diesel connection in this process.
///
/// Call this once during test setup so connections created by the code under test are
/// instrumented without any call-site changes.
pub fn install_default() -> diesel::QueryResult<()> {
    set_default_instrumentation(build_default_instrumentation)
}

fn build_default_instrumentation() -> Option<Box<dyn Instrumentation>> {
    Some(Box::new(AllureInstrumentation::new()))
}

fn sql_preview(sql: &str, max_sql_preview: usize) -> String {
    if sql.chars().count() <= max_sql_preview {
        return sql.to_string();
    }
    let truncated: String = sql.chars().take(max_sql_preview).collect();
    format!("{truncated}…")
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
