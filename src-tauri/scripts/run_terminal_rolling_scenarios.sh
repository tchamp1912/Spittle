#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCENARIO_DIR="${ROOT_DIR}/tests/rolling_scenarios"
EXPECT_SCRIPT="${ROOT_DIR}/scripts/replay_terminal_scenario.expect"

if ! command -v expect >/dev/null 2>&1; then
  echo "expect is required but not installed" >&2
  exit 1
fi
if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required but not installed" >&2
  exit 1
fi

normalize() {
  local s="$1"
  # Match harness behavior: collapse repeated whitespace + remove spaces before punctuation.
  s="$(printf "%s" "$s" | tr '\n' ' ' | awk '{$1=$1; print}')"
  printf "%s" "$s" | sed -E 's/[[:space:]]+([,.;:!?])/\1/g'
}

pass=0
fail=0

for scenario in "${SCENARIO_DIR}"/*.json; do
  [ -e "$scenario" ] || continue

  name="$(jq -r '.name' "$scenario")"
  expected_raw="$(jq -r '.hypotheses[-1]' "$scenario")"
  expected="$(normalize "$expected_raw")"

  hist_file="$(mktemp)"
  # Keep hist file around only for this scenario run.
  "${EXPECT_SCRIPT}" "$scenario" "$hist_file" >/dev/null 2>&1 || true

  # Use last non-empty, non-exit line as the typed command.
  actual_raw="$(grep -vE '^(exit|[[:space:]]*)$' "$hist_file" | tail -n 1 || true)"
  actual="$(normalize "$actual_raw")"
  rm -f "$hist_file"

  if [[ "$actual" == "$expected" ]]; then
    echo "PASS  ${name}"
    pass=$((pass + 1))
  else
    echo "FAIL  ${name}"
    echo "  expected: ${expected}"
    echo "  actual:   ${actual}"
    fail=$((fail + 1))
  fi
done

echo
echo "Summary: ${pass} passed, ${fail} failed"
if [[ "$fail" -gt 0 ]]; then
  exit 1
fi
