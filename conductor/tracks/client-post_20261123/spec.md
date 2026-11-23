# Spec — a resilient JSON POST (all-Rust Stage A, order)

order-service calls inventory-service (reserve, release) and payment-service (authorize)
with POSTs, through purerest's `Resilience` as its GETs would be. `TracedClient::send`
makes any method with an optional JSON body (traced `<METHOD> <host>`, metered with its
method); `get` is `send(GET)`. `ResilientClient::post_json` retries and breaks as `get`.

Acceptance: tests red first (no `post_json`), then green; clippy clean.
