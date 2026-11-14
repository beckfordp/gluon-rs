// SPDX-License-Identifier: MIT

//! The throwaway cluster: the `user` superuser with password `user` over TCP,
//! database `user`, as gluon-local's user-postgres.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::net::TcpStream;
use std::thread::sleep;
use std::time::{Duration, Instant};

use service_testing::{Postgres, free_port};

#[test]
fn a_free_port_has_no_listener() {
    assert!(TcpStream::connect(("127.0.0.1", free_port())).is_err());
}

#[test]
fn a_started_cluster_answers_queries_on_its_port() {
    let postgres = Postgres::start();
    assert_eq!(
        postgres.query("SELECT current_user || '@' || current_database()"),
        "user@user\n"
    );
    assert!(TcpStream::connect(("127.0.0.1", postgres.port)).is_ok());
}

#[test]
fn env_names_the_cluster_as_the_services_read_it() {
    let postgres = Postgres::start();
    assert_eq!(
        postgres.env(),
        vec![
            ("POSTGRES_HOST", "127.0.0.1".to_owned()),
            ("POSTGRES_PORT", postgres.port.to_string()),
            ("POSTGRES_DB", "user".to_owned()),
            ("POSTGRES_USER", "user".to_owned()),
            ("POSTGRES_PASSWORD", "user".to_owned()),
        ]
    );
}

#[test]
fn a_stopped_cluster_starts_again_with_its_data() {
    let postgres = Postgres::start();
    postgres.query("CREATE TABLE kept (id int)");
    postgres.stop();
    assert!(TcpStream::connect(("127.0.0.1", postgres.port)).is_err());
    postgres.start_again();
    assert_eq!(postgres.query("SELECT count(*) FROM kept"), "0\n");
}

#[test]
fn a_paused_cluster_accepts_nothing_until_resumed() {
    let postgres = Postgres::start();
    postgres.pause();
    let (done, finished) = std::sync::mpsc::channel();
    let port = postgres.port;
    std::thread::spawn(move || {
        // The connection is accepted by the kernel, but no startup reply comes.
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(500)))
            .unwrap();
        std::io::Write::write_all(&mut stream, &[0, 0, 0, 8, 4, 210, 22, 47]).unwrap();
        let answered = std::io::Read::read(&mut stream, &mut [0_u8; 1]).is_ok();
        done.send(answered).unwrap();
    });
    assert!(!finished.recv().unwrap(), "a paused cluster answered");
    postgres.resume();
    let deadline = Instant::now() + Duration::from_secs(10);
    while postgres.query("SELECT 1") != "1\n" {
        assert!(Instant::now() < deadline);
        sleep(Duration::from_millis(50));
    }
}

#[test]
fn pg_dump_shows_one_tables_schema() {
    let postgres = Postgres::start();
    postgres.query("CREATE TABLE dumped (id int PRIMARY KEY)");
    postgres.query("CREATE TABLE other (id int)");
    let dump = postgres.pg_dump("dumped");
    assert!(dump.contains("CREATE TABLE public.dumped"), "{dump}");
    assert!(!dump.contains("public.other"), "{dump}");
    assert!(!dump.contains("OWNER TO"), "{dump}");
}
