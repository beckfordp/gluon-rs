#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# CI for gluon-rs, defined once: the sandbox runs this script directly and
# GitHub Actions will call it. Stops at the first failing step; each step
# announces itself with a "== " line. The steps are scripts/lib/ci.sh's, which
# the Rust services' CI runs too.
# shellcheck source-path=SCRIPTDIR
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
# shellcheck source=lib/ci.sh
. scripts/lib/ci.sh

# Indented: its own "== " and SKIP lines aren't CI's.
step "script library tests"
scripts/test-lib.sh | sed 's/^/     /'

ci_fmt
ci_clippy
# kafka-client's consume loop needs a broker, which CI hasn't: the contract
# harness runs it (user-service's probe group, notification-service's consumer
# groups). Everything else in the crate is measured.
ci_coverage 'kafka-client/src/stream\.rs'
# sqlx's mysql and sqlite crates are in Cargo.lock only (.cargo/audit.toml).
ci_audit rsa libsqlite3-sys
