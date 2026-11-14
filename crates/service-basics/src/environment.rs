// SPDX-License-Identifier: MIT

//! The process environment as the config contract reads it: a map of the
//! variables whose name and value are both UTF-8.

use std::collections::HashMap;
use std::ffi::OsString;

/// This process's environment.
#[must_use]
pub fn variables() -> HashMap<String, String> {
    from_os(std::env::vars_os())
}

/// The UTF-8 pairs of `vars`; a name or value that isn't UTF-8 drops the pair.
pub fn from_os(vars: impl IntoIterator<Item = (OsString, OsString)>) -> HashMap<String, String> {
    vars.into_iter()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
        .collect()
}
