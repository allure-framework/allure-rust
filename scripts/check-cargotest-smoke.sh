#!/usr/bin/env bash
set -euo pipefail

results_dir="${ALLURE_SMOKE_RESULTS_DIR:-target/allure-cargotest-smoke-results}"
expected_file="${ALLURE_SMOKE_EXPECTED_FULLNAMES:-smokes/allure-cargotest/expected-fullnames.txt}"
runner="${ALLURE_SMOKE_RUNNER:-cargo}"

case "$results_dir" in
  /* | [A-Za-z]:/* | [A-Za-z]:\\*) ;;
  *) results_dir="$(pwd -P)/$results_dir" ;;
esac

rm -rf "$results_dir"
mkdir -p "$results_dir"

case "$runner" in
  cargo)
    ALLURE_RESULTS_DIR="$results_dir" cargo test --manifest-path smokes/allure-cargotest/Cargo.toml
    ;;
  nextest)
    # Runs one process per test, so each result file name must stay unique across processes.
    ALLURE_RESULTS_DIR="$results_dir" cargo nextest run --manifest-path smokes/allure-cargotest/Cargo.toml
    ;;
  *)
    echo "Unknown ALLURE_SMOKE_RUNNER: $runner (expected 'cargo' or 'nextest')" >&2
    exit 1
    ;;
esac

result_list="$(mktemp)"
actual_file="$(mktemp)"
trap 'rm -f "$result_list" "$actual_file"' EXIT

find "$results_dir" -type f -name '*-result.json' | sort > "$result_list"
if ! grep -q . "$result_list"; then
  echo "No Allure result files were generated in $results_dir" >&2
  exit 1
fi

while IFS= read -r result_file; do
  jq -r '.fullName // empty' "$result_file"
done < "$result_list" | sort > "$actual_file"

diff -u "$expected_file" "$actual_file"

result_count="$(wc -l < "$result_list" | tr -d '[:space:]')"
echo "Allure cargotest smoke reported $result_count expected tests."
