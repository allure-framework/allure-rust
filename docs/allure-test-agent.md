# Allure Test Agent

Use Allure agent mode to design, review, validate, debug, and enrich tests in this project.

This file is project-specific guidance. Durable test-design, expectation, and evidence rules live in the `allure-test-agent` skill. If the skill is available, use it together with this file. If the skill is unavailable, follow this file as the local fallback and keep conclusions conservative.

## Review Principle

Runtime first, source second.

- If a command executes tests and its result will be used for smoke checking, reasoning, review, coverage analysis, debugging, or any user-facing conclusion, run it through `allure agent`.
- Use agent-mode execution for smoke checks too, even when the change is small or mechanical.
- Only skip agent mode when it is impossible or when debugging agent mode itself.
- If agent output is missing or incomplete, debug that first and treat console-only conclusions as provisional.

## Local Capability Snapshot

Refresh this section when Allure, test runners, Allure results paths, Allure report generation, CI, or project wrappers change. Confirm local support with `allure --version`, `allure agent --help`, and `allure agent capabilities --json` before using optional commands.

Do not store the exact Allure version here. Version output is a runtime fact; this file stores the wrapper, last snapshot marker, and how to refresh capabilities.

- Allure wrapper: `allure`
- Capability snapshot last checked: 2026-06-12
- Refresh capabilities with: `allure --version`, `allure agent --help`, and `allure agent capabilities --json`
- Agent execution: supported with `allure agent [options] -- <command>`
- Output option: supported with `--output <dir>` or `-o <dir>`; default agent output is an automatic temp directory
- Expectation controls: inline `--goal`, `--expect-tests`, `--expect-label`, `--expect-env`, `--expect-test`, `--expect-prefix`, `--forbid-label`, `--expect-step-containing`, `--expect-steps`, `--expect-attachments`, `--expect-attachment`; file mode with `--expectations <file>`
- Latest/state directory recovery: `allure agent latest [--cwd <dir>]`, `allure agent state-dir [--cwd <dir>]`; `ALLURE_AGENT_STATE_DIR` can override state location
- Selection/rerun support: `allure agent select`, `allure agent query`, `allure agent --rerun-latest`, and `allure agent --rerun-from <output-dir>` are supported
- Discovery/configuration commands: agent discovery/configuration commands are unsupported; inspect Cargo, source, README files, and CI directly
- Local agent test service: unsupported in this repository; use the CLI wrapper directly

## Local Test Surfaces

- Test frameworks and runners: Rust `cargo test`; `allure-cargotest` macros and runtime wrappers; Tokio in smoke fixtures; shell smoke script for the excluded sample project
- Workspace packages: `allure-rust-commons`, `allure-reqwest`, `allure-test-macros`, `allure-cargotest`
- Test roots: `crates/**/src/*_tests.rs`, `crates/allure-cargotest/tests/e2e.rs`, and `smokes/allure-cargotest/tests/*.rs`
- Smoke project: `smokes/allure-cargotest` is excluded from the workspace and is exercised by `scripts/check-cargotest-smoke.sh` and by cargotest e2e tests that copy samples into temp projects
- Allure results paths: default `target/allure-results`; override with `ALLURE_RESULTS_DIR`; smoke script uses `ALLURE_SMOKE_RESULTS_DIR`
- Known selector support: Cargo package selection with `-p`, test-name filters after `--`, feature selection with `--features`, and Allure test-plan reruns transported with `ALLURE_TESTPLAN_PATH`
- Known environments or services needed for tests: local filesystem, nested Cargo runs for cargotest e2e, local HTTP servers for reqwest tests

## Allure Integrations

- Existing Allure integrations: `allure-rust-commons` runtime and writer APIs, `allure-cargotest` `#[allure_test]`/`#[step]`/`#[log_asserts]`, `allure-reqwest` HTTP exchange integration, Allure CLI in CI
- Runner config files: `Cargo.toml`, crate `Cargo.toml` files, `allurerc.mjs`, `.github/workflows/ci.yml`, and `scripts/check-cargotest-smoke.sh`
- Result-path configuration: `ALLURE_RESULTS_DIR` or default `target/allure-results`; smoke script uses `ALLURE_SMOKE_RESULTS_DIR`
- Supported integration configuration targets: `[package.metadata.allure]`, `[package.metadata.allure.labels]`, and `[[package.metadata.allure.modules]]` in Cargo manifests
- Validation command for integration setup: focused package tests through `allure agent`, full workspace through `allure agent`, and smoke script through `allure agent`
- Known unsupported or skipped integrations: local agent service and agent-side integration configuration are unsupported
- Integration-specific quirks: `allure-reqwest` has a non-default `middleware` feature; default workspace tests do not include that feature profile

## Project Test-Design Conventions

- Accepted test layers: unit tests for commons and macros, integration/e2e tests for cargotest sample behavior, HTTP integration tests for reqwest
- Preferred assertion style: standard Rust assertions; use `#[log_asserts]` or assertion logging from `#[allure_test]` when the test shape supports it, rather than hand-writing assertion-only steps
- Parameterized test style: prefer explicit tests and readable cases over loops when intent would be hidden
- Boring-test preference: keep tests direct; do not add a single manual step around an entire test body just to create evidence; shared helpers may handle mechanics but not per-test intent
- Smoke coverage conventions: `scripts/check-cargotest-smoke.sh` verifies smoke full names against `smokes/allure-cargotest/expected-fullnames.txt`
- Mocking and integration-test preference: prefer local deterministic boundaries already used by the suite, such as local HTTP servers and temp sample projects
- Explicit skip/assumption mechanics: unknown
- Suppression/quarantine policy: unknown

## Run Profiles

Use `allure agent` output defaults unless a temporary framework results directory is needed. `ALLURE_RESULTS_DIR` is confirmed by the project and should point at a per-run temp directory when the run result will be reviewed.

| Profile | Command or service intent | Expected use | Confidence limits |
| --- | --- | --- | --- |
| workspace | `TMP_DIR="$(mktemp -d)" && ALLURE_RESULTS_DIR="$TMP_DIR/allure-results" allure agent --goal "<goal>" -- cargo test --workspace --all-targets` | Broad default workspace validation | Does not include non-default feature profiles such as `allure-reqwest/middleware`; smoke project is excluded from the workspace |
| package | `TMP_DIR="$(mktemp -d)" && ALLURE_RESULTS_DIR="$TMP_DIR/allure-results" allure agent --goal "<goal>" -- cargo test -p <package> --all-targets` | Focused crate validation | Choose package and expectation controls for the touched scope |
| reqwest middleware | `TMP_DIR="$(mktemp -d)" && ALLURE_RESULTS_DIR="$TMP_DIR/allure-results" allure agent --goal "<goal>" -- cargo test -p allure-reqwest --all-targets --features middleware` | Validate the optional middleware integration | Only covers the middleware feature profile plus default reqwest tests |
| cargotest e2e | `TMP_DIR="$(mktemp -d)" && ALLURE_RESULTS_DIR="$TMP_DIR/allure-results" allure agent --goal "<goal>" -- cargo test -p allure-cargotest --all-targets` | Validate macro/test-plan/e2e sample behavior | Nested Cargo runs can be slower and may need dependency cache access |
| smoke script | `TMP_DIR="$(mktemp -d)" && ALLURE_SMOKE_RESULTS_DIR="$TMP_DIR/smoke-allure-results" allure agent --goal "<goal>" -- bash ./scripts/check-cargotest-smoke.sh` | Validate excluded smoke project full-name contract | Covers the smoke project, not the full workspace |

Add `--expect-tests <count>` or other expectation flags when the exact intended scope is known and the count is current for the change.

## Execution Signal And CI Trust

Do not present ignored, excluded, swallowed, advisory, or non-gating test execution as proof that behavior is safe.

- Default local test command: `cargo test --workspace --all-targets`
- Default local command exclusions: `smokes/allure-cargotest` is excluded from the Cargo workspace; optional feature profile `allure-reqwest --features middleware` is not included by default
- CI test jobs: `.github/workflows/ci.yml` runs format, clippy, regular tests with Allure, cargotest smoke check, report generation, report artifact upload, and PR report summary for non-fork PRs
- CI gating status: workflow runs on push and pull request; branch protection requirements are unknown
- Known ignored, skipped, muted, quarantined, or disabled tests: unknown
- Test artifacts retained by CI: Allure test and smoke dump zip files for 7 days; generated Allure report artifact for 7 days

If CI or local execution is non-gating, excludes important tests, or swallows failures, call that out before using the run as proof.

Do not hide missing or unsupported coverage behind runtime `if` branches, early returns, conditional test registration, or helper aliases. Use explicit skip, conditional-skip, assumption, xfail, quarantine, or setup-failure conventions when the project defines them; otherwise report the limitation.

## Local Expectation Controls

Before each validation run, decide whether expectations reduce a real risk for the intended conclusion. When they do, use the smallest fresh inline options supported by `allure agent --help`.

- Supported expectation mechanism: inline CLI controls and advanced YAML/JSON file mode
- Exact test/file/suite/label/profile support: full names with `--expect-test`, prefixes with `--expect-prefix`, labels with `--expect-label`, environments with `--expect-env`, counts with `--expect-tests`
- Excluded-scope controls: forbidden labels with `--forbid-label`; forbidden environments/full names are unsupported
- Evidence expectation controls: `--expect-step-containing`, `--expect-steps`, `--expect-attachments`, and `--expect-attachment`
- Check/assertion step-name controls: assertion logging can expose standard assertion names; use evidence expectations only when the expected step names are stable
- Broad-audit fallback: run the narrowest practical command, review observed scope from manifests, and state scope limits

Prefer inline options. Use `--expectations <file>` only as advanced mode when the contract is too large, generated, or policy-controlled.

When expectations are justified, they should state only the parts that matter for this run:

- what claim or validation depth the run is meant to support
- what should run
- what should not run
- which profile, environment, variant, or parameter set is intended
- what important checks or evidence should be visible through supported reporting or documented step-name conventions
- why this scope is enough
- what the run cannot prove

Treat the run goal as a claim boundary for review, not as proof. If the goal is wrong or stale, keep the runtime evidence and report what the observed run actually supports.

## Core Loops

### Test Review Loop

1. Identify the exact review scope and validation depth.
2. Create the smallest meaningful expectations using local supported controls when they protect the review conclusion.
3. Run only that scope through `allure agent`.
4. Print the run's `index.md` path.
5. Review `index.md`, `manifest/run.json`, `manifest/test-events.jsonl`, `manifest/tests.jsonl`, `manifest/findings.jsonl`, and relevant per-test markdown.
6. Inspect source code only after runtime evidence explains what executed.
7. Call out weak scope, weak evidence, execution-signal limits, or partial runtime modeling.

### Test Authoring Loop

1. Understand the feature, issue, expected behavior, and risk.
2. Read the `allure-test-agent` skill's test-design guidance when available.
3. Create the smallest meaningful expectations for the intended scope when they reduce a real validation risk.
4. Write or update focused tests without weakening useful coverage.
5. Run the intended scope through agent mode.
6. Review scope, checks, evidence, and execution signal before claiming validation.
7. Enrich tests when evidence is weak, then rerun with fresh agent output.

### Evidence And Metadata Enrichment Loop

Use this when tests pass but are hard to review:

1. Identify weak evidence, missing checks, missing setup state, missing artifacts, or noisy metadata.
2. Prefer framework integrations, assertion logging, useful attachments, and helper-boundary instrumentation for mechanics over wrapping every line.
3. Add useful steps, attachments, parameters, descriptions, labels, or links using project conventions.
4. Keep per-test intent metadata inline with each test.
5. Redact sensitive values while preserving useful artifact shape.
6. Rerun the same intended scope and report evidence changes.

### Coverage Review Loop

1. Split broad audits into scoped groups when practical.
2. Ensure each group has distinct agent output and use expectations only when the group has a known scope or supports a validation conclusion.
3. Run each group through agent mode.
4. Separate observed runtime coverage from inferred source-code coverage.
5. Mark review incomplete until every scoped group was validated through matched expectations, reviewed observed scope, or documented as a broad package-health audit.

## Runtime Artifact Review

After each agent-mode run:

- print the run's `index.md` path
- read `manifest/run.json`
- read `manifest/test-events.jsonl`
- read `manifest/tests.jsonl`
- read `manifest/findings.jsonl`
- read relevant per-test markdown before inspecting source
- inspect global stderr/log artifacts when runner-visible failures are not represented as logical tests

## Output, State, And Reruns

Do not create persistent agent output or expectation paths. Modern `allure agent` creates and prints a temp output directory when no output is provided; use that default unless a specific path is needed. Prefer `--output` for explicit paths.

Allure results paths such as `target/allure-results` or `<tmp>/allure-results` are separate reporting configuration. Do not use framework result variables such as `ALLURE_RESULTS_DIR` as agent-output controls. Use `ALLURE_RESULTS_DIR` only for framework result emission, and keep the final directory name `allure-results` when the results must be discovered by Allure.

- Agent output policy: CLI-provided temp directory by default; explicit `--output` only when useful for organizing groups
- Latest output recovery: `allure agent latest [--cwd <dir>]`
- State directory override: `ALLURE_AGENT_STATE_DIR=<dir>`
- Rerun from latest/prior output: `allure agent --rerun-latest -- <command>` or `allure agent --rerun-from <output-dir> -- <command>`
- Selection/test plan support: `allure agent select` writes test-plan JSON; reruns use `ALLURE_TESTPLAN_PATH`
- Parallel-run rule: output paths and expectation state must not be shared
- CI artifact retention: Allure dump zip artifacts and generated report artifacts are retained for 7 days

## Project Metadata Conventions

Per-test metadata belongs inline with the test. Do not centralize descriptions, labels, links, parameters, or intent-defining step names in helper wrappers, lookup tables, current-test-name mappings, or generated registries. Reusable helpers may handle mechanics only; if a helper sets metadata, every metadata value must be passed explicitly from that test.

- Feature/story/component/service labels: use project APIs such as `feature`, `story`, `label`, `layer`, and package metadata labels when they already exist
- Owner/team metadata: owner labels exist in sample coverage; no global ownership policy is defined
- Severity or priority metadata: severity labels exist in sample coverage; no global severity policy is defined
- Issue, bug, requirement, or known-defect links: issue and TMS link helpers exist; project policy is unknown
- Suite/package/module taxonomy: synthetic suite labels derive from Rust module paths; Cargo metadata can add package/module labels
- Parameter naming and dynamic-history exclusions: runtime parameter APIs exist; exclude dynamic values from history when they do not define the scenario
- Metadata to avoid: decorative labels, unused taxonomies, and metadata hidden away from the individual test site

## Project Evidence Conventions

- Test descriptions: concise behavior intent, expected result, or regression reason; keep descriptions inline with each test
- Attachments: use structured content types for JSON and rich Allure HTTP exchange attachments for HTTP traffic; for parser, serializer, command, fixture, and generated-file tests, attach the raw input and the meaningful output/result when it helps explain the assertion
- Step naming: concrete behavior boundaries such as setup, parsing, command execution, request/response handling, and cleanup; do not add ceremonial whole-test wrapper steps
- Check/assertion step naming: standard assertion logging may report assertion macro names; do not replace logged assertions with hand-written verification steps
- Assertion/check visibility: use assertion logging when useful so expected and actual values stay visible; disable noisy assertion logging with `[package.metadata.allure] log_asserts = false` or `ALLURE_LOG_ASSERTS=false` only when there is other meaningful evidence
- Fixture/setup evidence: temp sample projects, generated Cargo manifests, test-plan JSON, HTTP exchanges, and result summaries are useful when relevant to the assertion
- Sensitive data redaction: redact tokens, auth headers, cookies, and secrets while preserving artifact shape

## Acceptance Rules

Accept a run only when:

- observed scope matches the intended scope, or drift is explained
- coverage remains meaningful for the stated conclusion
- important checks are visible through supported reporting, documented step-name conventions, or source review covers the gap
- evidence is strong enough to explain what happened
- execution-signal limits are explicit
- no high-confidence placeholder or noop evidence findings remain
- partial runtime modeling is called out

Console-only conclusions are provisional when agent output is absent or incomplete.
