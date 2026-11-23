# Spec — routes that skip poison (all-Rust Stage A, order)

order-service's Scala consumers decode outside the DLQ guard: an undecodable envelope or
payload is logged and skipped, only a handler failure (a store down) is dead-lettered.
`Routes::DEAD_LETTERS_POISON` (default true, purekafka's `EnvelopeHandler`) says which;
false logs the same error line and commits past.

Acceptance: a test red first (no such const), then green; clippy clean.
