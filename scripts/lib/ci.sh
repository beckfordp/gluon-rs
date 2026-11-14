# shellcheck shell=bash source-path=SCRIPTDIR
# SPDX-License-Identifier: MIT
# The CI steps every gluon Rust repo runs, defined once (gluon TD-43.2). A
# repo's scripts/ci.sh sets -euo pipefail, cd's to its root, sources this file
# and calls the steps in order, with its own between them. Each step announces
# itself with a "== " line; user-service-rs's scripts/verify-rust-ci.sh
# matches the names.
# shellcheck source=portable.sh
. "$(dirname "${BASH_SOURCE[0]}")/portable.sh"

step() { echo "== $*"; }

ci_fmt() {
  step "cargo fmt --check"
  cargo fmt --all --check
}

ci_clippy() {
  step "cargo clippy (warnings are errors)"
  cargo clippy --workspace --all-targets --locked -- -D warnings
}

# Runs the tests once, instrumented; fails below 80% of lines (workflow.md).
# cargo-llvm-cov 0.9.1 exits 1 below the gate without a word, so say why here.
ci_coverage() {
  step "tests with coverage (lines >= 80%)"
  local log status=0
  log="$(mktemp)"
  cargo llvm-cov --workspace --locked --fail-under-lines 80 --summary-only 2>&1 \
    | tee "$log" || status=$?
  if [ "$status" != 0 ] && ! grep -q "test result: FAILED" "$log"; then
    local lines
    lines="$(awk '/^TOTAL/ {print $(NF-3)}' "$log")"
    echo "FAIL coverage: ${lines:-an unknown share} of lines covered, below the 80% gate"
  fi
  rm -f "$log"
  return "$status"
}

# ci_audit [CRATE...]: cargo audit, after checking that each CRATE, whose
# advisory .cargo/audit.toml ignores, is only in Cargo.lock, never built. The
# RustSec advisory database lives on GitHub, which the sandbox can't reach
# (gluon plans/private/sandbox.md): say so rather than pass silently.
ci_audit() {
  step "cargo audit"
  local advisory_db="${CI_ADVISORY_DB:-https://github.com/rustsec/advisory-db}"
  if ! with_timeout 20 git ls-remote --exit-code "$advisory_db" HEAD > /dev/null 2>&1; then
    echo "SKIP cargo-audit: advisory database unreachable ($advisory_db)"
    return 0
  fi
  if ! command -v cargo-audit > /dev/null 2>&1; then
    echo "SKIP cargo-audit: not installed (GitHub Actions installs it)"
    return 0
  fi
  local crate tree
  for crate in "$@"; do
    tree="$(cargo tree -i "$crate" -e all --target all --locked -q)" || return 1
    if [ -n "$tree" ]; then
      echo "FAIL $crate is built: drop its .cargo/audit.toml ignore"
      return 1
    fi
  done
  cargo audit --deny warnings
}
