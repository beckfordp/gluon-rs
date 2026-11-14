# Plan — shared CI and verify scripts, Linux and macOS (gluon TD-43.2)

## Phase 1: the script library [checkpoint: f7a315a]
- [x] Task: `portable.sh`, `ci.sh`, `verify.sh` and their tests 51331c5
    - [x] Red: `scripts/test-lib.sh`: with_timeout's status, 124 and prompt
          return (also under `$(…)`), owner_of, step, ci_audit's SKIP,
          verify.sh's ok/bad/expect counts
    - [x] Green: the library; gluon-rs `scripts/ci.sh` on it
- [x] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)
