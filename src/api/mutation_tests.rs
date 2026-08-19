use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    thread,
    time::Duration,
};

use super::{Client, CreateSession, Prompt};

#[test]
fn session_mutations_match_the_server_contract() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        for index in 0..6 {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            let (expected, body, status) = match index {
                0 => (
                    "POST /session?directory=%2Fworkspace ",
                    session_json("ses_new", "new"),
                    "200 OK",
                ),
                1 => (
                    "PATCH /session/ses_new?directory=%2Fworkspace ",
                    session_json("ses_new", "renamed"),
                    "200 OK",
                ),
                2 => (
                    "GET /session/ses_new/children?directory=%2Fworkspace ",
                    "[]".into(),
                    "200 OK",
                ),
                3 => (
                    "POST /session/ses_new/prompt_async?directory=%2Fworkspace ",
                    String::new(),
                    "204 No Content",
                ),
                4 => (
                    "POST /session/ses_new/abort?directory=%2Fworkspace ",
                    "true".into(),
                    "200 OK",
                ),
                _ => (
                    "DELETE /session/ses_new?directory=%2Fworkspace ",
                    "true".into(),
                    "200 OK",
                ),
            };
            assert!(
                request.starts_with(expected),
                "unexpected request: {request}"
            );
            if index == 0 {
                assert!(request.contains(r#"{"parentID":"ses_parent","title":"new"}"#));
            }
            if index == 1 {
                assert!(request.contains(r#"{"title":"renamed"}"#));
            }
            if index == 3 {
                assert!(request.contains(r#""type":"text","text":"hello""#));
            }
            write_response(&mut stream, status, &body);
        }
    });
    let client = test_client(address);

    pollster::block_on(async {
        let created = client
            .create_session(CreateSession {
                parent_id: Some("ses_parent".into()),
                title: Some("new".into()),
            })
            .await
            .unwrap();
        assert_eq!(created.id, "ses_new");
        assert_eq!(
            client
                .rename_session("ses_new", "renamed")
                .await
                .unwrap()
                .title,
            "renamed"
        );
        assert!(client.children("ses_new").await.unwrap().is_empty());
        client
            .prompt(
                "ses_new",
                Prompt {
                    text: "hello".into(),
                    model: None,
                    agent: None,
                },
            )
            .await
            .unwrap();
        assert!(client.abort_session("ses_new").await.unwrap());
        assert!(client.delete_session("ses_new").await.unwrap());
    });
    server.join().unwrap();
}

fn read_request(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut chunk = [0; 4096];
    loop {
        let count = stream.read(&mut chunk).unwrap();
        bytes.extend_from_slice(&chunk[..count]);
        let text = String::from_utf8_lossy(&bytes);
        let Some(header_end) = text.find("\r\n\r\n") else {
            continue;
        };
        let content_length = text[..header_end]
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length: ")
                    .map(str::to_owned)
            })
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        if bytes.len() >= header_end + 4 + content_length {
            return String::from_utf8(bytes).unwrap();
        }
    }
}

fn write_response(stream: &mut TcpStream, status: &str, body: &str) {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
}

fn session_json(id: &str, title: &str) -> String {
    format!(
        r#"{{"id":"{id}","projectID":"prj","directory":"/workspace","title":"{title}","version":"1.18.16","time":{{"created":1,"updated":2}}}}"#
    )
}

fn test_client(address: SocketAddr) -> Client {
    Client::new(
        &format!("http://{address}"),
        Some("/workspace".into()),
        None,
        None,
    )
    .unwrap()
}
