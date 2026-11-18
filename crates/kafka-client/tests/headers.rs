// SPDX-License-Identifier: MIT

//! A record's headers as the consume span reads them (`traceparent`): text
//! headers kept in order; one with no value or no UTF-8 value dropped.

use kafka_client::headers::text_headers;

#[test]
fn text_headers_are_kept_in_order() {
    let raw: [(&str, Option<&[u8]>); 2] = [
        ("traceparent", Some(b"00-4bf9-00f0-01")),
        ("correlation_id", Some(b"u1")),
    ];
    assert_eq!(
        text_headers(raw),
        vec![
            ("traceparent".to_owned(), "00-4bf9-00f0-01".to_owned()),
            ("correlation_id".to_owned(), "u1".to_owned()),
        ]
    );
}

#[test]
fn a_header_without_a_text_value_is_dropped() {
    let raw: [(&str, Option<&[u8]>); 3] = [
        ("empty", None),
        ("binary", Some(&[0xff, 0xfe])),
        ("kept", Some(b"v")),
    ];
    assert_eq!(text_headers(raw), vec![("kept".to_owned(), "v".to_owned())]);
}
