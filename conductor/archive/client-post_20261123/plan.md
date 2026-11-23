# Plan

- [x] Task 1: tests red (E0599 no `post_json`, `send`), `TracedClient::send`, `ResilientClient::post_json`/`send`; `get` is `send(GET)`; 13 passed, CI green 44ce190
- [x] Review: no findings (one request path for every method; the GET spans, metrics and retry unchanged, their tests pass as before)
