// SPDX-License-Identifier: MIT

//! The throwaway Redis (gluon all-Rust Stage A P0.3): it answers, stops and
//! comes back on the same port.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Read, Write};
use std::net::TcpStream;

use service_testing::Redis;

fn ping(port: u16) -> std::io::Result<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.write_all(b"*1\r\n$4\r\nPING\r\n")?;
    let mut reply = [0u8; 7];
    stream.read_exact(&mut reply)?;
    Ok(String::from_utf8_lossy(&reply).into_owned())
}

#[test]
fn it_answers_stops_and_comes_back() {
    let redis = Redis::start();
    assert_eq!(ping(redis.port).unwrap(), "+PONG\r\n");
    assert_eq!(redis.url(), format!("redis://127.0.0.1:{}", redis.port));
    redis.stop();
    assert!(ping(redis.port).is_err(), "still answering after stop");
    redis.start_again();
    assert_eq!(ping(redis.port).unwrap(), "+PONG\r\n");
}
