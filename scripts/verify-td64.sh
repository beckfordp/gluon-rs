#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Verifies track kafka-client-td64_20261119 (gluon TD-64) up to phase N, each check
# printed PASS or FAIL; the phase-completion check that replaces a walkthrough:
#   phase 1  no Result<_, String> left in kafka-client or kafka-event-consumer; the
#            typed-error tests pass (breaker open, publish failure, ConsumeError)
#   phase 2  the injected clock gives the age; the commit runs off the async worker
#   phase 3  metrics built once; GroupId and Topic parse as Kafka allows
#   every    scripts/ci.sh green (fmt, clippy, tests, coverage >= 80%)
# Usage: scripts/verify-td64.sh [N]   (default 3). Exits 1 on any FAIL.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
PHASE="${1:-3}"
LOGS="$(mktemp -d "${TMPDIR:-/tmp}/verify-td64.XXXXXX")"
echo "TD-64 up to phase $PHASE — logs in $LOGS"
FAILED=0

# check LABEL COMMAND...: PASS when COMMAND exits 0; its output kept in the logs.
check() {
  local label="$1" log
  shift
  log="$LOGS/$(echo "$label" | tr -c 'a-z0-9' '-').log"
  if "$@" > "$log" 2>&1; then
    echo "PASS $label"
  else
    echo "FAIL $label (log: $log)"
    FAILED=$((FAILED + 1))
  fi
}

# tests PACKAGE TEST...: each named test ran and passed (not filtered away).
tests() {
  local package="$1" name output
  shift
  for name in "$@"; do
    output="$(cargo test -q -p "$package" -- --exact "$name" 2>&1)" || { echo "$output"; return 1; }
    echo "$output"
    case "$output" in
      *" 1 passed"*) ;;
      *) echo "$name: not run"; return 1 ;;
    esac
  done
}

no_string_results() {
  ! grep -rn 'Result<[^>]*String>' crates/kafka-client crates/kafka-event-consumer
}

if [ "$PHASE" -ge 1 ]; then
  check "no Result<_, String> in the Kafka crates" no_string_results
  check "typed dead-letter errors" tests kafka-event-consumer \
    a_failed_dlq_publish_fails_the_guard an_open_breaker_fails_the_guard_without_publishing \
    a_failed_dead_letter_is_the_streams_error
  check "ConsumeError reads as before, keeps its cause" tests kafka-client \
    a_failed_commit_says_so_and_keeps_its_cause broker_and_subscribe_failures_read_as_rdkafkas \
    a_failed_dead_letter_is_the_streams_error
fi
check "scripts/ci.sh" bash scripts/ci.sh

if [ "$FAILED" = 0 ]; then
  echo "PASS: TD-64 phase $PHASE verified"
else
  echo "FAIL: $FAILED check(s) failed"
  exit 1
fi
