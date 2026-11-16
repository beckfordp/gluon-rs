// SPDX-License-Identifier: MIT

//! Values as the JVM prints them on the wire, for the Scala services' JSON
//! (circe encodes a `java.time.Instant` with its `toString`).

use std::fmt::Write as _;

use time::{OffsetDateTime, UtcOffset};

/// How `java.time.Instant.toString` prints an instant: UTC, seconds always,
/// the fraction in 3, 6 or 9 digits as needed, then `Z` (user-service capture
/// `http/users-200.txt`, notification-service capture `http/emails-all.txt`).
#[must_use]
pub fn java_instant(value: OffsetDateTime) -> String {
    let utc = value.to_offset(UtcOffset::UTC);
    let mut text = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        utc.year(),
        u8::from(utc.month()),
        utc.day(),
        utc.hour(),
        utc.minute(),
        utc.second()
    );
    let nanos = utc.nanosecond();
    let _ = if nanos == 0 {
        Ok(())
    } else if nanos.is_multiple_of(1_000_000) {
        write!(text, ".{:03}", nanos / 1_000_000)
    } else if nanos.is_multiple_of(1_000) {
        write!(text, ".{:06}", nanos / 1_000)
    } else {
        write!(text, ".{nanos:09}")
    };
    text.push('Z');
    text
}
