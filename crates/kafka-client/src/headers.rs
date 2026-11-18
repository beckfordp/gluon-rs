// SPDX-License-Identifier: MIT

//! A record's headers as text, for the consume span (`traceparent`).

/// The headers with a UTF-8 value, in order; one with no value, or a value
/// that isn't UTF-8, is dropped (no span context can come from it).
pub fn text_headers<'a>(
    raw: impl IntoIterator<Item = (&'a str, Option<&'a [u8]>)>,
) -> Vec<(String, String)> {
    raw.into_iter()
        .filter_map(|(key, value)| Some((key.to_owned(), String::from_utf8(value?.to_vec()).ok()?)))
        .collect()
}
