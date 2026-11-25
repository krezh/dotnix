use std::{
    collections::HashMap,
    io::{Cursor, Read, Write},
    net::TcpListener,
    thread,
};

use tungstenite::{
    Message, WebSocket,
    handshake::server::Request,
    protocol::Role,
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
        let (conn, _) = listener.accept().unwrap();
        let mut ws = tungstenite::accept_hdr(conn, |request: &Request, response| {
            assert_eq!(request.uri().path(), "/stream");
            assert_eq!(request.headers()["X-Gotify-Key"], "secret-token");
            Ok(response)
        })
        .unwrap();
        ws.send(Message::Ping("hi".into())).unwrap();
        assert_eq!(ws.read().unwrap(), Message::Pong("hi".into()));

        let part1 = br#"{"id":99,"appid":3,"title":"Alert","#;
        let part2 = br#""message":"Hello","priority":8,"date":"2026-10-25T16:00:00Z"}"#;
        let raw = ws.get_mut();
        raw.write_all(&[0x01, part1.len() as u8]).unwrap();
        raw.write_all(part1).unwrap();
        raw.write_all(&[0x80, part2.len() as u8]).unwrap();
        raw.write_all(part2).unwrap();

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

        // 3. GET /message?limit=200
        let (mut msg_conn, _) = listener.accept().unwrap();
        let msg_req = read_http_request(&mut msg_conn);
        assert!(msg_req.starts_with("GET /message?limit=200 HTTP/1.1\r\n"));
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

    let client = std::net::TcpStream::connect(addr).unwrap();
    let mut socket = handshake(client, &endpoint, "secret-token").unwrap();
    let raw = read_message(&mut socket).unwrap();
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
fn follows_every_page_of_messages() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let endpoint = Endpoint::parse(&format!("ws://{addr}")).unwrap();

    let server = thread::spawn(move || {
        let pages: [(&str, &str); 2] = [
            (
                "GET /message?limit=200 HTTP/1.1\r\n",
                r#"{"messages":[{"id":9}],"paging":{"next":"/message?limit=200&since=9","since":9}}"#,
            ),
            (
                "GET /message?limit=200&since=9 HTTP/1.1\r\n",
                r#"{"messages":[{"id":4}],"paging":{"since":4}}"#,
            ),
        ];
        for (request, body) in pages {
            let (mut conn, _) = listener.accept().unwrap();
            assert!(read_http_request(&mut conn).starts_with(request));
            write!(
                conn,
                "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
    });

    let messages = fetch_messages(&api_agent(), &endpoint, "token", &HashMap::new()).unwrap();
    let ids: Vec<u64> = messages.iter().map(|entry| entry.id).collect();
    assert_eq!(ids, vec![9, 4]);
    server.join().unwrap();
}

/// What a server sent, as the client's end of an open stream.
fn received(frames: Vec<u8>) -> WebSocket<Cursor<Vec<u8>>> {
    WebSocket::from_raw_socket(Cursor::new(frames), Role::Client, None)
}

#[test]
fn rejects_invalid_fragmented_frame_sequences() {
    // A continuation frame without a first fragment.
    assert!(read_message(&mut received(vec![0x80, 0x02, b'h', b'i'])).is_err());
    // A new message before the one split over frames has finished.
    let interleaved = vec![
        0x01, 0x04, b'p', b'a', b'r', b't', 0x81, 0x04, b'n', b'e', b'x', b't',
    ];
    assert!(read_message(&mut received(interleaved)).is_err());
}

#[test]
fn reads_a_message_whose_first_fragment_is_empty() {
    let body = br#"{"id":7}"#;
    let mut frames = vec![0x01, 0x00, 0x80, body.len() as u8];
    frames.extend_from_slice(body);
    let raw = read_message(&mut received(frames)).unwrap();
    assert_eq!(raw.id, 7);
}
