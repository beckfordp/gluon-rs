#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Tests scripts/lib/ on whichever OS runs it: Linux (the sandbox, GitHub
# Actions) and macOS (bash 3.2, BSD tools, no coreutils timeout). Each result
# is printed as it is found; the exit status is 0 only if all pass.
#   scripts/test-lib.sh
# shellcheck source-path=SCRIPTDIR
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit 1
# shellcheck source=lib/verify.sh
. scripts/lib/verify.sh
# shellcheck source=lib/ci.sh
. scripts/lib/ci.sh

# within NAME SECONDS: ok if no more than SECONDS have passed since $start
within() {
  if [ $((SECONDS - start)) -le "$2" ]; then ok "$1"; else bad "$1: took $((SECONDS - start)) s"; fi
}

echo "== with_timeout =="
with_timeout 5 sh -c 'exit 3'
expect "the command's status" 3 "$?"
start=$SECONDS
with_timeout 1 sleep 10
expect "124 when it runs longer" 124 "$?"
within "stopped it within the limit" 3
start=$SECONDS
out="$(with_timeout 10 echo hi)"
expect "the command's output" hi "$out"
within "returns when the command ends" 2
bash -ec '. scripts/lib/portable.sh; with_timeout 1 sleep 10'
expect "124 under set -e too" 124 "$?"

echo "== owner_of =="
expect "the owner of the repo" "$(id -un)" "$(owner_of "$PWD")"

echo "== step =="
expect "announces itself" "== cargo fmt --check" "$(step cargo fmt --check)"

echo "== ci_audit =="
# shellcheck disable=SC2119 # no lockfile-only crates
out="$(CI_ADVISORY_DB=http://127.0.0.1:9/advisory-db ci_audit 2>&1)"
expect "says why it skipped" $'== cargo audit\nSKIP cargo-audit: advisory database unreachable (http://127.0.0.1:9/advisory-db)' "$out"
# A crate cargo tree can't find fails the step rather than passing it.
stand_ins="$(mktemp -d)"
printf '#!/bin/sh\n' > "$stand_ins/cargo-audit"
chmod +x "$stand_ins/cargo-audit"
cargo() { if [ "$1" = tree ]; then return 101; fi; }
# shellcheck disable=SC2329 # called through with_timeout
git() { return 0; }
out="$(PATH="$stand_ins:$PATH" && set -e && ci_audit no-such-crate 2>&1)"
expect "fails when cargo tree does" 1 "$?"
unset -f cargo git
rm -rf "$stand_ins"

echo "== ci_coverage =="
# A stand-in cargo prints its arguments: the pattern reaches llvm-cov, or nothing does.
cargo() { echo "args: $*"; }
# shellcheck disable=SC2119 # no pattern: everything measured
out="$(ci_coverage 2>&1)"
expect "no pattern, no ignore flag" "" "$(grep -o -- '--ignore-filename-regex.*' <<< "$out")"
out="$(ci_coverage 'probe/src/(main|run)\.rs' 2>&1)"
expect "the pattern passed on" "--ignore-filename-regex probe/src/(main|run)\.rs" "$(grep -o -- '--ignore-filename-regex.*' <<< "$out")"
unset -f cargo

echo "== verify.sh's counts =="
counts="$(
  VERIFY_PASSED=0 VERIFY_FAILED=0
  ok one > /dev/null
  bad two > /dev/null
  check three true > /dev/null
  check four false > /dev/null
  expect five a b > /dev/null
  echo "$VERIFY_PASSED $VERIFY_FAILED"
)"
expect "passed and failed" "2 3" "$counts"

verify_summary "scripts/lib"
