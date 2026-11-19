# Spec — envelope parity on invalid input

Track `envelope-parity_20261118`, gluon **RPN-11-could-fix** (with
notification-service-rs's `rust-idioms-could-fix_20261118`); gluon
TECHNICAL-DEBT.md item 9.

- **S-3:** Scala's `EventEnvelope.timestamp` is an `Instant`: an envelope whose
  timestamp isn't one fails to decode, so it is poison — dead-lettered raw after
  the guard's retries. gluon-rs keeps a `String`, so it is handled. Decode it as
  an instant (`time::OffsetDateTime`, RFC 3339: Java's `ISO_INSTANT` as the
  capture and the producers write it; Java also accepts an omitted seconds field,
  which no producer writes).
- **S-4:** Scala's consumers pass `record.key.getOrElse("")` to the DLQ guard, so
  a null-key record is dead-lettered with the key `""`; gluon-rs publishes no
  key. `EventConsumer::process` gives a null key as `""`.

No capture or harness record has a bad timestamp or a null key, so the checks
are Rust tests here; the contract is regression (user-service's probe group,
notification-service's user-events group). Acceptance: gluon
`bin/verify-complete-RPN-11.sh` criteria 1, 2, 6.
