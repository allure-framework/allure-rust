# allure-diesel

`allure-diesel` records executed [Diesel](https://diesel.rs) queries as Allure steps. It hooks
into Diesel's `Instrumentation` API, so it is wired up once in test or setup code and never
touches production query call sites.

## Add the crate

```bash
cargo add allure-diesel --dev
```

## Per-connection wiring

Attach the instrumentation to a connection after establishing it. Every query executed on that
connection is then recorded against whichever Allure test is running on the current thread.

```rust
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

fn connect() -> ConnectionResult<SqliteConnection> {
    let mut conn = SqliteConnection::establish(":memory:")?;
    conn.set_instrumentation(allure_diesel::AllureInstrumentation::new());
    Ok(conn)
}
```

## Global wiring

`install_default` registers the instrumentation for every newly established connection in the
process. Call it once during test setup so connections created by the code under test are
captured with no call-site changes.

```rust
fn setup() -> diesel::QueryResult<()> {
    allure_diesel::install_default()
}
```

## What is captured

- Each executed statement becomes a step named with its SQL, carrying the full SQL as a
  `query.sql` attachment.
- `BEGIN` / `COMMIT` / `ROLLBACK` become `transaction` steps that nest their inner queries.
- A query that returns an error marks its step failed with the Diesel error message.

## Capture options

```rust
use allure_diesel::{AllureInstrumentation, CaptureOptions};

let options = CaptureOptions::default()
    .without_transactions()       // don't record transaction boundaries
    .with_connection_events()     // record connection establishment (off by default)
    .without_sql_attachment()     // keep the SQL in the step name only
    .with_max_sql_preview(256);   // truncate the SQL used in step names

let instrumentation = AllureInstrumentation::with_options(options);
```

## Limitations

Diesel's instrumentation exposes the SQL of each statement but **not the returned rows**, and no
other non-invasive Diesel hook does — so this crate records queries and transactions, not result
sets. The rendered SQL may contain sensitive bind values; keep that in mind before publishing
reports.
