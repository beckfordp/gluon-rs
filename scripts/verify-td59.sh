#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Verifies track user-client-td59_20261119 (gluon TD-59) up to phase N, each check
# printed PASS or FAIL; the phase-completion check that replaces a walkthrough:
#   phase 1  user-client answers each outcome as notification-service's UserClient
#   phase 2  events-probe looks users up only through user-client (one copy)
#   every    scripts/ci.sh green (fmt, clippy, tests, coverage >= 80%)
# Usage: scripts/verify-td59.sh [N]   (default 2). Exits 1 on any FAIL.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
PHASE="${1:-2}"
LOGS="$(mktemp -d "${TMPDIR:-/tmp}/verify-td59.XXXXXX")"
echo "TD-59 up to phase $PHASE — logs in $LOGS"
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

PROBE=../../services/user-service-rs/crates/events-probe

# one_copy: no user-service request built in the probe; it depends on user-client.
one_copy() {
  ! grep -rn '/users/' "$PROBE/src" && grep -n '^user-client' "$PROBE/Cargo.toml"
}

if [ "$PHASE" -ge 1 ]; then
  check "user-client answers as UserClient does" tests user-client \
    a_found_user_is_their_email_from_get_users_id an_unknown_or_malformed_id_is_no_user \
    any_other_status_fails_with_the_users_id_and_the_status a_body_that_isnt_a_user_fails \
    no_answer_fails_with_the_clients_error the_service_constructor_builds_its_own_client
fi
if [ "$PHASE" -ge 2 ]; then
  check "one copy: events-probe uses user-client" one_copy
fi
check "scripts/ci.sh" bash scripts/ci.sh

if [ "$FAILED" = 0 ]; then
  echo "PASS: TD-59 phase $PHASE verified"
else
  echo "FAIL: $FAILED check(s) failed"
  exit 1
fi
