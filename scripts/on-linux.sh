#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# The Linux test bed (gluon TD-43.2): runs COMMAND in REPO_PATH, a repo in
# gluon's layout, on Linux in Docker, from the laptop. The sandbox and GitHub
# Actions run the Rust repos' scripts on Linux, the laptop on macOS; a script
# change is shown on both before it lands.
#   scripts/on-linux.sh REPO_PATH COMMAND...
#   scripts/on-linux.sh services/user-service-rs scripts/verify-rust-ci.sh
# The repos are read, never written: gluon is mounted read-only and copied
# into the volume gluon-rs-linux-bed-work (lib/gluon-rs, registry, REPO_PATH;
# no target/ or .git), whose target/ dirs stay between runs. Cargo's downloads
# stay in gluon-rs-linux-bed-cargo. The image is built when its Dockerfile
# (scripts/linux-bed/) changes. COMMAND's exit status is this script's
# (125-127 can also be docker's own). Runs queue, one at a time, since they
# share the volume. The copies have no .git: a script that reads git history
# doesn't run here. GLUON overrides where gluon is (default: the one holding
# lib/gluon-rs).
set -euo pipefail
if [ $# -lt 2 ]; then
  echo "usage: $0 REPO_PATH COMMAND..." >&2
  exit 2
fi
BED="$(cd "$(dirname "${BASH_SOURCE[0]}")/linux-bed" && pwd)"
GLUON="${GLUON:-$(cd "$BED/../../../.." && pwd)}"
repo="${1%/}"
shift
if [ ! -d "$GLUON/$repo" ]; then
  echo "FAIL no $repo under $GLUON" >&2
  exit 2
fi

image="gluon-rs-linux-bed:$(cksum < "$BED/Dockerfile" | cut -d' ' -f1)"
if ! docker image inspect "$image" > /dev/null 2>&1; then
  echo "== building $image (once per Dockerfile change)" >&2
  docker build -q -t "$image" "$BED" >&2
fi

tty=""
if [ -t 0 ] && [ -t 1 ]; then tty=-t; fi
# shellcheck disable=SC2016 # expanded in the container
exec docker run --rm -i ${tty:+"$tty"} \
  -v "$GLUON:/host:ro" \
  -v gluon-rs-linux-bed-work:/work \
  -v gluon-rs-linux-bed-cargo:/home/tester/.cargo \
  -e REPO="$repo" \
  "$image" flock /work/.bed.lock bash -c '
    set -e
    for path in lib/gluon-rs registry "$REPO"; do
      mkdir -p "/work/$path"
      rsync -a --delete --exclude=/target/ --exclude=/.git/ "/host/$path/" "/work/$path/"
    done
    cd "/work/$REPO"
    exec "$@"' bash "$@"
