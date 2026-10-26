use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpListener,
    thread,
};

use super::{
    api_agent, delete_all_messages, delete_message, fetch_applications, fetch_messages, handshake,
    read_message,
};
use crate::features::gotify::state::Endpoint;

fn read_http_request(stream: &mut impl Read) -> String {
    let mut req = Vec::new();
    let mut byte = [0_u8; 1];
    while !req.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte).unwrap();
        req.push(byte[0]);
    }
    String::from_utf8(req).unwrap()
}

#[test]
fn upgrades_answers_pings_reads_frames_and_calls_rest_endpoints() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let endpoint = Endpoint::parse(&format!("ws://{addr}")).unwrap();

    let server = thread::spawn(move || {
        // 1. WebSocket /stream handshake + Ping/Pong + fragmented message
        let (mut ws, _) = listener.accept().unwrap();
        let req = read_http_request(&mut ws);
        assert!(req.starts_with("GET /stream HTTP/1.1\r\n"));
        assert!(req.contains("X-Gotify-Key: secret-token\r\n"));
        ws.write_all(
            b"HTTP/1.1 101 Switching Protocols\r\n\
              Upgrade: websocket\r\n\
              Connection: Upgrade\r\n\r\n",
        )
        .unwrap();

        ws.write_all(&[0x89, 2, b'h', b'i']).unwrap();
        let mut pong = [0_u8; 8];
        ws.read_exact(&mut pong).unwrap();
        assert_eq!(pong[0], 0x8A);
        assert_eq!(pong[1], 0x82);
        let mask = &pong[2..6];
        let payload: Vec<u8> = pong[6..8]
            .iter()
            .enumerate()
            .map(|(i, b)| b ^ mask[i])
            .collect();
        assert_eq!(payload, b"hi");

        let part1 = br#"{"id":99,"appid":3,"title":"Alert","#;
        let part2 = br#""message":"Hello","priority":8,"date":"2026-10-25T16:00:00Z"}"#;
        ws.write_all(&[0x01, part1.len() as u8]).unwrap();
        ws.write_all(part1).unwrap();
        ws.write_all(&[0x80, part2.len() as u8]).unwrap();
        ws.write_all(part2).unwrap();

        // 2. GET /application
        let (mut app_conn, _) = listener.accept().unwrap();
        let app_req = read_http_request(&mut app_conn);
        assert!(app_req.starts_with("GET /application HTTP/1.1\r\n"));
        let app_body = br#"[{"id":3,"name":"Gatus"}]"#;
        write!(
            app_conn,
            "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
            app_body.len()
        )
        .unwrap();
        app_conn.write_all(app_body).unwrap();

        // 3. GET /message?limit=50
        let (mut msg_conn, _) = listener.accept().unwrap();
        let msg_req = read_http_request(&mut msg_conn);
        assert!(msg_req.starts_with("GET /message?limit=50 HTTP/1.1\r\n"));
        let msg_body =
            br#"{"messages":[{"id":99,"appid":3,"title":"Alert","message":"Hello","priority":8}]}"#;
        write!(
            msg_conn,
            "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
            msg_body.len()
        )
        .unwrap();
        msg_conn.write_all(msg_body).unwrap();

        // 4. DELETE /message/99
        let (mut del_conn, _) = listener.accept().unwrap();
        let del_req = read_http_request(&mut del_conn);
        assert!(del_req.starts_with("DELETE /message/99 HTTP/1.1\r\n"));
        del_conn
            .write_all(b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 0\r\n\r\n")
            .unwrap();

        // 5. DELETE /message
        let (mut clear_conn, _) = listener.accept().unwrap();
        let clear_req = read_http_request(&mut clear_conn);
        assert!(clear_req.starts_with("DELETE /message HTTP/1.1\r\n"));
        clear_conn
            .write_all(b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 0\r\n\r\n")
            .unwrap();
    });

    let mut client = std::net::TcpStream::connect(addr).unwrap();
    handshake(&mut client, &endpoint, "secret-token").unwrap();
    let raw = read_message(&mut client).unwrap();
    assert_eq!(raw.id, 99);
    assert_eq!(raw.appid, 3);
    assert_eq!(raw.priority, 8);

    let agent = api_agent();
    let apps = fetch_applications(&agent, &endpoint, "secret-token").unwrap();
    assert_eq!(apps, HashMap::from([(3, "Gatus".to_owned())]));
    let messages = fetch_messages(&agent, &endpoint, "secret-token", &apps).unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].app, "Gatus");
    delete_message(&agent, &endpoint, "secret-token", 99).unwrap();
    delete_all_messages(&agent, &endpoint, "secret-token").unwrap();
    server.join().unwrap();
}

#[test]
fn rejects_invalid_fragmented_frame_sequences() {
    use std::io::Cursor;

    // Unexpected continuation frame without preceding fragment
    let mut bad_continuation = Cursor::new(vec![0x80, 0x02, b'h', b'i']);
    let err = read_message(&mut bad_continuation).unwrap_err();
    assert!(
        err.contains("unexpected continuation frame"),
        "expected continuation error, got: {err}"
    );

    // New message arriving before prior fragmented sequence completed
    let mut interleaved = Cursor::new(vec![
        0x01, 0x04, b'p', b'a', b'r', b't', 0x81, 0x04, b'n', b'e', b'x', b't',
    ]);
    let err = read_message(&mut interleaved).unwrap_err();
    assert!(
        err.contains("new message arrived before prior fragment finished"),
        "expected interleaved error, got: {err}"
    );
}
