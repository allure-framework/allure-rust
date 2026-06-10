#!/usr/bin/env bash
set -euo pipefail

results_dir="${ALLURE_SAMPLE_RESULTS_DIR:-target/allure-sample-smoke-results}"
expected_file="${ALLURE_SAMPLE_EXPECTED_FULLNAMES:-crates/allure-cargotest/tests/samples/expected-fullnames.txt}"

case "$results_dir" in
  /* | [A-Za-z]:/* | [A-Za-z]:\\*) ;;
  *) results_dir="$(pwd -P)/$results_dir" ;;
esac

rm -rf "$results_dir"
mkdir -p "$results_dir"

ALLURE_RESULTS_DIR="$results_dir" cargo test -p allure-cargotest-ci-samples

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
echo "Sample smoke reported $result_count expected tests."
