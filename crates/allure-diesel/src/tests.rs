use super::*;
use allure_cargotest::{allure_test, CargoTestReporter};
use diesel::prelude::*;
use diesel::sql_query;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(QueryableByName, Debug)]
struct User {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    id: i32,
    #[diesel(sql_type = diesel::sql_types::Text)]
    name: String,
}

diesel::table! {
    dao_users (id) {
        id -> Integer,
        name -> Text,
    }
}

#[derive(Queryable, Debug)]
struct DaoUser {
    id: i32,
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = dao_users)]
struct NewDaoUser<'a> {
    id: i32,
    name: &'a str,
}

// A service-style helper that issues two queries: one write followed by one read.
fn create_and_fetch(conn: &mut SqliteConnection, id: i32, name: &str) -> QueryResult<Vec<DaoUser>> {
    diesel::insert_into(dao_users::table)
        .values(&NewDaoUser { id, name })
        .execute(conn)?;
    dao_users::table.filter(dao_users::name.eq(name)).load(conn)
}

fn make_results_dir(test_name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "allure-diesel-tests-{test_name}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos()
    ))
}

// Runs `body` under a dedicated Allure test context (its own results dir), with an in-memory
// SQLite connection whose activity is captured by `AllureInstrumentation`. Returns the emitted
// test result JSON plus the results directory so attachment files can be read back.
fn run_within_test_context<F>(test_name: &str, body: F) -> (Value, PathBuf)
where
    F: FnOnce(&mut SqliteConnection),
{
    let out_dir = make_results_dir(test_name);
    let reporter = CargoTestReporter::new(&out_dir).expect("reporter should initialize");
    let full_name = format!("allure_diesel::tests::{test_name}");

    reporter.run_test_with_metadata(test_name, Some(&full_name), None, None, |_allure| {
        let mut conn =
            SqliteConnection::establish(":memory:").expect("in-memory sqlite should connect");
        conn.set_instrumentation(AllureInstrumentation::new());
        body(&mut conn);
    });

    (read_result(&out_dir, test_name), out_dir)
}

fn read_result(out_dir: &Path, test_name: &str) -> Value {
    for entry in fs::read_dir(out_dir).expect("results dir should exist") {
        let path = entry.expect("dir entry should be readable").path();
        let is_result = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.ends_with("-result.json"))
            .unwrap_or(false);
        if !is_result {
            continue;
        }
        let value: Value = serde_json::from_str(
            &fs::read_to_string(&path).expect("result json should be readable"),
        )
        .expect("result json should parse");
        if value["name"] == test_name {
            return value;
        }
    }
    panic!("expected result for {test_name} was not written");
}

fn read_attachment(out_dir: &Path, step: &Value) -> String {
    let source = step["attachments"][0]["source"]
        .as_str()
        .expect("step should carry an attachment source");
    fs::read_to_string(out_dir.join(source)).expect("attachment file should be readable")
}

fn steps(result: &Value) -> &Vec<Value> {
    result["steps"]
        .as_array()
        .expect("steps should be an array")
}

fn find_step<'a>(steps: &'a [Value], prefix: &str) -> &'a Value {
    steps
        .iter()
        .find(|step| {
            step["name"]
                .as_str()
                .map(|name| name.starts_with(prefix))
                .unwrap_or(false)
        })
        .unwrap_or_else(|| panic!("expected a step starting with {prefix:?}"))
}

#[allure_test]
#[test]
fn records_each_query_as_step_with_sql_attachment() {
    let (result, out_dir) =
        run_within_test_context("records_each_query_as_step_with_sql_attachment", |conn| {
            sql_query("create table users (id integer primary key, name text not null)")
                .execute(conn)
                .expect("create table should succeed");
            sql_query("insert into users (id, name) values (1, 'Ada')")
                .execute(conn)
                .expect("insert should succeed");
            let users: Vec<User> = sql_query("select id, name from users order by id")
                .load(conn)
                .expect("select should succeed");
            assert_eq!(users.len(), 1);
            assert_eq!(users[0].id, 1);
            assert_eq!(users[0].name, "Ada");
        });

    let steps = steps(&result);
    find_step(steps, "create table users");
    find_step(steps, "insert into users");

    let select = find_step(steps, "select id, name from users");
    assert_eq!(select["status"], "passed");
    // The full SQL is attached via a nested `query.sql` step under the query step.
    let sql_step = &select["steps"][0];
    assert_eq!(sql_step["name"], "query.sql");
    assert_eq!(sql_step["attachments"][0]["name"], "query.sql");
    // Diesel's DebugQuery renders the statement plus a non-stable `-- binds: [...]` annotation.
    assert!(
        read_attachment(&out_dir, sql_step).starts_with("select id, name from users order by id")
    );
}

#[allure_test]
#[test]
fn marks_failed_query_step_failed() {
    let (result, _out_dir) = run_within_test_context("marks_failed_query_step_failed", |conn| {
        let outcome = sql_query("select id from missing_table").execute(conn);
        assert!(
            outcome.is_err(),
            "query against a missing table should fail"
        );
    });

    let failed = find_step(steps(&result), "select id from missing_table");
    assert_eq!(failed["status"], "failed");
    assert!(
        failed["statusDetails"]["message"]
            .as_str()
            .map(|message| !message.is_empty())
            .unwrap_or(false),
        "failed query step should carry the Diesel error message"
    );
}

#[allure_test]
#[test]
fn records_transaction_with_nested_query_steps() {
    let (result, _out_dir) =
        run_within_test_context("records_transaction_with_nested_query_steps", |conn| {
            sql_query("create table t (id integer primary key)")
                .execute(conn)
                .expect("create table should succeed");
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                sql_query("insert into t (id) values (1)").execute(conn)?;
                sql_query("insert into t (id) values (2)").execute(conn)?;
                Ok(())
            })
            .expect("transaction should commit");
        });

    let steps = steps(&result);
    let transaction = find_step(steps, "transaction");
    let nested = transaction["steps"]
        .as_array()
        .expect("transaction step should have nested steps");
    let inserts = nested
        .iter()
        .filter(|step| {
            step["name"]
                .as_str()
                .map(|name| name.starts_with("insert into t"))
                .unwrap_or(false)
        })
        .count();
    assert_eq!(inserts, 2, "both inserts should nest under the transaction");

    // The COMMIT boundary nests under the transaction step and is not a top-level sibling.
    assert!(
        nested
            .iter()
            .any(|step| step["name"].as_str().unwrap_or("").starts_with("COMMIT")),
        "the COMMIT boundary should nest under the transaction step"
    );
    assert!(
        steps
            .iter()
            .all(|step| !step["name"].as_str().unwrap_or("").starts_with("COMMIT")),
        "COMMIT should not appear as a top-level sibling"
    );
}

#[allure_test]
#[test]
fn records_typed_query_builder_calls_as_sibling_steps() {
    let (result, out_dir) = run_within_test_context(
        "records_typed_query_builder_calls_as_sibling_steps",
        |conn| {
            sql_query("create table dao_users (id integer primary key, name text not null)")
                .execute(conn)
                .expect("create table should succeed");
            // A service method issuing multiple queries records each as its own step.
            let found = create_and_fetch(conn, 1, "Ada").expect("dao calls should succeed");
            assert_eq!(found.len(), 1);
            assert_eq!(found[0].id, 1);
            assert_eq!(found[0].name, "Ada");
        },
    );

    let steps = steps(&result);
    // Each query the service issued is a distinct top-level sibling step, none nested in another.
    assert_eq!(
        steps.len(),
        3,
        "create + insert + select should be three sibling steps"
    );
    // The query builder renders these itself, so the keywords are Diesel's own uppercase output
    // (unlike the lowercase raw SQL the other tests pass to `sql_query`).
    let insert = find_step(steps, "INSERT INTO");
    assert_eq!(insert["status"], "passed");
    let select = find_step(steps, "SELECT");
    assert_eq!(select["status"], "passed");

    // The parameterized SELECT captures its bind value in the query.sql attachment.
    let sql_step = &select["steps"][0];
    assert_eq!(sql_step["name"], "query.sql");
    assert!(
        read_attachment(&out_dir, sql_step).contains("Ada"),
        "the query.sql attachment should include the rendered bind value"
    );
}

#[allure_test]
#[test]
fn records_rolled_back_transaction_and_recovers() {
    let (result, _out_dir) =
        run_within_test_context("records_rolled_back_transaction_and_recovers", |conn| {
            sql_query("create table t (id integer primary key)")
                .execute(conn)
                .expect("create table should succeed");
            let outcome = conn.transaction::<(), diesel::result::Error, _>(|conn| {
                sql_query("insert into t (id) values (1)").execute(conn)?;
                Err(diesel::result::Error::RollbackTransaction)
            });
            assert!(outcome.is_err(), "transaction should roll back");
            // A query issued after the rollback must record at the top level again.
            sql_query("insert into t (id) values (2)")
                .execute(conn)
                .expect("post-rollback insert should succeed");
        });

    let steps = steps(&result);
    let transaction = find_step(steps, "transaction");
    let nested = transaction["steps"]
        .as_array()
        .expect("transaction step should have nested steps");
    assert!(
        nested.iter().any(|step| step["name"]
            .as_str()
            .unwrap_or("")
            .starts_with("insert into t")),
        "the in-transaction insert should nest under the transaction step"
    );
    // The ROLLBACK boundary nests under the transaction step rather than leaking out as a sibling.
    assert!(
        nested
            .iter()
            .any(|step| step["name"].as_str().unwrap_or("").starts_with("ROLLBACK")),
        "the ROLLBACK boundary should nest under the transaction step"
    );
    assert!(
        steps
            .iter()
            .all(|step| !step["name"].as_str().unwrap_or("").starts_with("ROLLBACK")),
        "ROLLBACK should not appear as a top-level sibling"
    );

    // The post-rollback insert is a top-level sibling, proving the guard stack rebalanced after
    // the rollback rather than leaving a phantom open transaction step.
    let top_level_inserts = steps
        .iter()
        .filter(|step| {
            step["name"]
                .as_str()
                .unwrap_or("")
                .starts_with("insert into t")
        })
        .count();
    assert_eq!(
        top_level_inserts, 1,
        "only the post-rollback insert should be a top-level step"
    );
}

#[allure_test]
#[test]
fn failed_commit_marks_transaction_step_failed() {
    let (result, _out_dir) = run_within_test_context(
        "failed_commit_marks_transaction_step_failed",
        |conn| {
            // A deferred foreign-key constraint is only checked at COMMIT, so the transaction
            // fails on the boundary query rather than on an inner statement.
            sql_query("pragma foreign_keys = on")
                .execute(conn)
                .expect("enabling foreign keys should succeed");
            sql_query("create table parent (id integer primary key)")
                .execute(conn)
                .expect("create parent should succeed");
            sql_query("create table child (id integer primary key, parent_id integer not null references parent(id) deferrable initially deferred)")
                .execute(conn)
                .expect("create child should succeed");
            let outcome = conn.transaction::<(), diesel::result::Error, _>(|conn| {
                sql_query("insert into child (id, parent_id) values (1, 999)").execute(conn)?;
                Ok(())
            });
            assert!(
                outcome.is_err(),
                "commit should fail on the deferred foreign-key violation"
            );
        },
    );

    let steps = steps(&result);
    let transaction = find_step(steps, "transaction");
    // The transaction step reflects the commit failure instead of being reported as passed.
    assert_eq!(transaction["status"], "failed");
    assert!(
        transaction["statusDetails"]["message"]
            .as_str()
            .map(|message| !message.is_empty())
            .unwrap_or(false),
        "the transaction step should carry the commit error message"
    );

    // The failing COMMIT is a nested boundary step, and is also marked failed.
    let nested = transaction["steps"]
        .as_array()
        .expect("transaction step should have nested steps");
    let commit = nested
        .iter()
        .find(|step| step["name"].as_str().unwrap_or("").starts_with("COMMIT"))
        .expect("the COMMIT boundary should nest under the transaction step");
    assert_eq!(commit["status"], "failed");

    // COMMIT must not leak out as a top-level sibling after the transaction step.
    assert!(
        steps
            .iter()
            .all(|step| !step["name"].as_str().unwrap_or("").starts_with("COMMIT")),
        "COMMIT should be nested under the transaction, not a top-level sibling"
    );
}

#[allure_test]
#[test]
fn failed_begin_closes_transaction_step_and_recovers() {
    let (result, _out_dir) = run_within_test_context(
        "failed_begin_closes_transaction_step_and_recovers",
        |conn| {
            sql_query("create table t (id integer primary key)")
                .execute(conn)
                .expect("create table should succeed");
            // Put SQLite into a transaction behind Diesel's back so the managed BEGIN (depth 1)
            // fails: "cannot start a transaction within a transaction".
            sql_query("begin")
                .execute(conn)
                .expect("raw begin should succeed");
            let outcome = conn.transaction::<(), diesel::result::Error, _>(|conn| {
                sql_query("insert into t (id) values (1)").execute(conn)?;
                Ok(())
            });
            assert!(
                outcome.is_err(),
                "BEGIN should fail while already in a transaction"
            );
            // A query issued after the failed start must record at the top level, proving the
            // transaction guard did not leak onto the stack.
            sql_query("insert into t (id) values (2)")
                .execute(conn)
                .expect("post-failure insert should succeed");
        },
    );

    let steps = steps(&result);
    let transaction = find_step(steps, "transaction");
    // The transaction step is closed with the BEGIN failure rather than left dangling.
    assert_eq!(transaction["status"], "failed");
    assert!(
        transaction["statusDetails"]["message"]
            .as_str()
            .map(|message| !message.is_empty())
            .unwrap_or(false),
        "the transaction step should carry the failed BEGIN error message"
    );
    // The failing BEGIN (Diesel's own uppercase boundary) nests under the transaction step.
    let nested = transaction["steps"]
        .as_array()
        .expect("transaction step should have nested steps");
    let begin = nested
        .iter()
        .find(|step| step["name"].as_str().unwrap_or("").starts_with("BEGIN"))
        .expect("the BEGIN boundary should nest under the transaction step");
    assert_eq!(begin["status"], "failed");

    // The post-failure insert is a top-level sibling: the guard stack rebalanced.
    let top_level_inserts = steps
        .iter()
        .filter(|step| {
            step["name"]
                .as_str()
                .unwrap_or("")
                .starts_with("insert into t")
        })
        .count();
    assert_eq!(
        top_level_inserts, 1,
        "only the post-failure insert should be a top-level step"
    );
}

#[allure_test]
#[test]
fn sql_preview_truncates_long_sql_with_ellipsis() {
    let long = format!("SELECT {}", "x".repeat(100));
    let preview = sql_preview(&long, 10);
    assert_eq!(
        preview.chars().count(),
        11,
        "10 characters plus one ellipsis"
    );
    assert!(preview.ends_with('…'));
    // SQL within the limit is returned unchanged.
    assert_eq!(sql_preview("short", 10), "short");
}
