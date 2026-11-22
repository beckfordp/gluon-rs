# Spec — any Redis command, bounded (all-Rust Stage A, cart)

cart-service keeps its carts in Redis (hashes, Lua scripts). `Cache::query` runs any
command with the same timeout, fail-fast and reconnect as get/set/delete; `redis` is
re-exported to build commands.

Acceptance: a test red first (no `query`), then green; clippy clean.
