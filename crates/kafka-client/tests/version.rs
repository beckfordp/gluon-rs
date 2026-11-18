// SPDX-License-Identifier: MIT

//! The crate builds, links, and reports its package version.

#[test]
fn version_is_the_package_version() {
    assert_eq!(kafka_client::version(), "0.1.0");
}
