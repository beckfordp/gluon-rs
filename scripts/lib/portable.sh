# shellcheck shell=bash source-path=SCRIPTDIR
# SPDX-License-Identifier: MIT
# Commands that differ between Linux and macOS (bash 3.2, BSD tools, no
# coreutils timeout), done one way that works on both. Sourced by ci.sh and
# verify.sh.

# with_timeout SECONDS COMMAND...: the command's status, or 124 if it runs
# longer and is stopped (as coreutils timeout).
with_timeout() {
  local seconds="$1"
  shift
  "$@" &
  local pid=$!
  # The watchdog exits 124 once it has stopped the command, 0 when stopped
  # itself. It holds no output, so `$(with_timeout …)` returns when the command
  # does; its sleep goes with it.
  (
    trap 'kill "$sleeper" 2> /dev/null; exit 0' TERM
    sleep "$seconds" &
    sleeper=$!
    wait "$sleeper"
    trap '' TERM
    kill -TERM "$pid" 2> /dev/null
    exit 124
  ) > /dev/null 2>&1 &
  local watchdog=$!
  local status=0 timed_out=0
  wait "$pid" 2> /dev/null || status=$? # quiet: bash reports a stopped job
  kill "$watchdog" 2> /dev/null
  wait "$watchdog" || timed_out=$?
  [ "$timed_out" = 124 ] && status=124
  return "$status"
}

# owner_of PATH: the user who owns PATH (GNU stat, else BSD stat).
owner_of() { stat -c %U "$1" 2> /dev/null || stat -f %Su "$1"; }
