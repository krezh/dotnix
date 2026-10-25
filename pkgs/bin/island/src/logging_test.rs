use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    sync::mpsc,
    thread,
    time::Duration,
};

use super::{Entry, HUB, lock, serve_client};

#[test]
fn hanging_up_releases_the_client_at_once() {
    let (mut client, server) = UnixStream::pair().unwrap();
    let id = u64::MAX;
    let (done_tx, done_rx) = mpsc::channel();
    thread::spawn(move || {
        serve_client(server, id);
        let _ = done_tx.send(());
    });

    writeln!(client, r#"{{"lines":0,"follow":true}}"#).unwrap();
    while !lock(&HUB.clients).contains_key(&id) {
        thread::sleep(Duration::from_millis(1));
    }
    HUB.publish(Entry {
        time: 0,
        level: "INFO".to_owned(),
        module: "test".to_owned(),
        message: "hello".to_owned(),
        fields: Vec::new(),
    });
    let mut line = String::new();
    BufReader::new(&client).read_line(&mut line).unwrap();
    assert!(line.contains("hello"));

    drop(client);
    assert!(done_rx.recv_timeout(Duration::from_millis(100)).is_ok());
    assert!(!lock(&HUB.clients).contains_key(&id));
}
