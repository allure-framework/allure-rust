# Project Guide

Use [Allure Test Agent](docs/allure-test-agent.md) for all test-related work in this repository.

- Read `docs/allure-test-agent.md` before designing, writing, reviewing, validating, debugging, or enriching tests. Use the `$allure-test-agent` skill as the durable behavior guide when it is installed; this project file contains local commands and conventions.
- If a command executes tests and its result will be used for smoke checking, reasoning, review, coverage analysis, debugging, or any user-facing conclusion, run it through `allure agent`.
- Use agent-mode execution for smoke checks too, even when the change is small or mechanical.
- Only skip agent mode when it is impossible or when you are debugging agent mode itself.
- If agent output is missing or incomplete, debug that first rather than silently falling back to console-only review.
- Use Allure agent mode when adding tests for features or fixes so expectations, evidence quality, and scope review are part of the loop.
- Do not present ignored, excluded, swallowed, or non-gating tests as a passing validation signal.
- Never create pull requests or push git branches without explicit confirmation from the user.
