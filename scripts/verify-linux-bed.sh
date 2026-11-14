#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Verifies track linux-bed_20261114: scripts/on-linux.sh runs a Rust repo's
# script on Linux from the laptop (Docker), passes its exit status through and
# leaves the host's repos as they were. Each result is printed as it is found.
#   scripts/verify-linux-bed.sh
# shellcheck source-path=SCRIPTDIR
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit 1
# shellcheck source=lib/verify.sh
. scripts/lib/verify.sh
GLUON="$(realpath ../..)"
SCRATCH="$(mktemp -d /tmp/verify-linux-bed.XXXXXX)"
trap 'rm -rf "$SCRATCH"' EXIT
host_state() { git status --porcelain && git -C "$GLUON" status --porcelain -- registry; }
before="$(host_state)"

echo "== 1. a script on the bed =="
if scripts/on-linux.sh lib/gluon-rs scripts/test-lib.sh > "$SCRATCH/test-lib.log" 2>&1; then
  ok "test-lib.sh on Linux: $(tail -1 "$SCRATCH/test-lib.log")"
else
  bad "test-lib.sh on Linux failed; its last lines:"
  tail -8 "$SCRATCH/test-lib.log" | sed 's/^/     /'
fi
expect "it ran on Linux" Linux "$(scripts/on-linux.sh lib/gluon-rs uname -s 2> /dev/null)"
expect "as tester, not root" tester "$(scripts/on-linux.sh lib/gluon-rs id -un 2> /dev/null)"

echo "== 2. the exit status passes through =="
scripts/on-linux.sh lib/gluon-rs sh -c 'exit 3' > /dev/null 2>&1
expect "exit 3" 3 "$?"

echo "== 3. runs queue: they share the volume =="
scripts/on-linux.sh lib/gluon-rs sleep 4 > /dev/null 2>&1 &
first=$!
sleep 1
start=$SECONDS
scripts/on-linux.sh lib/gluon-rs true > /dev/null 2>&1
waited=$((SECONDS - start))
wait "$first"
if [ "$waited" -ge 2 ]; then ok "a second run waited for the first (${waited} s)"; else bad "a second run ran alongside the first (${waited} s)"; fi

echo "== 4. the host's repos are untouched =="
expect "gluon-rs and gluon's registry unchanged" "$before" "$(host_state)"

verify_summary "the Linux test bed"
