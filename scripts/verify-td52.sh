#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Verifies track http-edge-td52_20261119 (gluon TD-52.3) up to phase N, each check
# printed PASS or FAIL; the phase-completion check that replaces a walkthrough:
#   phase 1  service-basics' edge: the CORS cases and the Allow-less 405
#   phase 2  neither port has CORS or Allow code of its own; both use edge()
#   every    scripts/ci.sh green (fmt, clippy, tests, coverage >= 80%)
# Usage: scripts/verify-td52.sh [N]   (default 2). Exits 1 on any FAIL.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
PHASE="${1:-2}"
LOGS="$(mktemp -d "${TMPDIR:-/tmp}/verify-td52.XXXXXX")"
echo "TD-52.3 up to phase $PHASE — logs in $LOGS"
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

PORTS=(../../services/user-service-rs ../../services/notification-service-rs)

# no_edge_code: neither port keeps a cors module or touches Allow.
no_edge_code() {
  local port found=0
  for port in "${PORTS[@]}"; do
    if ls "$port"/crates/*/src/cors.rs 2> /dev/null; then found=1; fi
    if grep -rn "header::ALLOW\b\|ALLOW;\|ACCESS_CONTROL_" "$port"/crates/*/src; then found=1; fi
  done
  [ "$found" = 0 ]
}

# both_use_edge: each port's server goes through service_basics::edge.
both_use_edge() {
  local port
  for port in "${PORTS[@]}"; do
    grep -rn "edge(" "$port"/crates/*/src/server.rs || return 1
  done
}

if [ "$PHASE" -ge 1 ]; then
  check "the edge answers as http4s and tapir do" tests service-basics \
    a_preflight_the_policy_allows_gets_every_allow_header \
    a_preflight_on_any_path_is_answered_by_the_policy \
    a_preflight_the_policy_refuses_gets_only_vary \
    a_cross_origin_request_gets_only_allow_origin \
    an_exposing_policy_names_its_headers_on_a_cross_origin_answer \
    no_route_means_no_cors_headers \
    an_options_request_that_isnt_a_preflight_is_routed_and_gets_vary \
    a_wrong_method_is_tapirs_405_without_allow
fi
if [ "$PHASE" -ge 2 ]; then
  check "no CORS or Allow code in either port" no_edge_code
  check "both ports use service_basics::edge" both_use_edge
fi
check "scripts/ci.sh" bash scripts/ci.sh

if [ "$FAILED" = 0 ]; then
  echo "PASS: TD-52.3 phase $PHASE verified"
else
  echo "FAIL: $FAILED check(s) failed"
  exit 1
fi
