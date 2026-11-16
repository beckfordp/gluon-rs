// SPDX-License-Identifier: MIT

//! `java.time.Instant.toString` formatting. Sources: user-service capture
//! `http/users-200.txt` (`2026-11-09T18:43:19.511602Z`), notification-service
//! capture `http/emails-all.txt`; the rest is `Instant.toString`'s contract
//! (3, 6 or 9 fraction digits, none for whole seconds).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use service_basics::jvm::java_instant;
use time::macros::datetime;

#[test]
fn instants_print_as_java_does() {
    for (instant, printed) in [
        (
            datetime!(2026-11-09 18:43:19.511602 UTC),
            "2026-11-09T18:43:19.511602Z",
        ),
        (
            datetime!(2026-11-09 18:43:19.511 UTC),
            "2026-11-09T18:43:19.511Z",
        ),
        (
            datetime!(2026-11-09 18:43:19.0005 UTC),
            "2026-11-09T18:43:19.000500Z",
        ),
        (datetime!(2026-11-09 18:43:00 UTC), "2026-11-09T18:43:00Z"),
        (
            datetime!(2026-11-09 19:43:19.511602 +01:00),
            "2026-11-09T18:43:19.511602Z",
        ),
        (
            datetime!(2026-11-09 18:43:19.123456789 UTC),
            "2026-11-09T18:43:19.123456789Z",
        ),
        (
            datetime!(2026-11-11 11:21:06.848580 UTC),
            "2026-11-11T11:21:06.848580Z",
        ),
    ] {
        assert_eq!(java_instant(instant), printed);
    }
}
