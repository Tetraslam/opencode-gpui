use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

use crate::event::Event;

use super::Client;

#[test]
fn bootstraps_against_the_server_contract() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        for _ in 0..3 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 2048];
            let length = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..length]);
            let body = if request.starts_with("GET /global/health ") {
                r#"{"healthy":true,"version":"1.18.16"}"#
            } else if request.starts_with("GET /session?directory=%2Fworkspace ") {
                r#"[{"id":"ses_1","projectID":"prj_1","directory":"/workspace","title":"first","version":"1.18.16","time":{"created":1,"updated":2}},{"id":"ses_2","projectID":"prj_1","directory":"/workspace","title":"second","version":"1.18.16","time":{"created":2,"updated":3}}]"#
            } else if request
                .starts_with("GET /session/ses_2/message?directory=%2Fworkspace&limit=100 ")
            {
                r#"[{"info":{"id":"msg_1","sessionID":"ses_2","role":"user","time":{"created":3},"agent":"build","model":{"providerID":"openai","modelID":"gpt-test"}},"parts":[{"id":"part_1","sessionID":"ses_2","messageID":"msg_1","type":"text","text":"hello"}]}]"#
            } else {
                panic!("unexpected request: {request}");
            };
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
    });
    let client = test_client(address);
    let bootstrap = pollster::block_on(client.bootstrap()).unwrap();
    assert!(bootstrap.health.healthy);
    assert_eq!(bootstrap.health.version, "1.18.16");
    assert_eq!(
        bootstrap
            .sessions
            .iter()
            .map(|session| session.id.as_str())
            .collect::<Vec<_>>(),
        ["ses_2", "ses_1"]
    );
    let messages = pollster::block_on(client.messages("ses_2", 100)).unwrap();
    assert_eq!(messages[0].parts[0].text(), Some("hello"));
    server.join().unwrap();
}

#[test]
fn streams_directory_scoped_events() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0; 2048];
        let length = stream.read(&mut request).unwrap();
        assert!(
            String::from_utf8_lossy(&request[..length])
                .starts_with("GET /event?directory=%2Fworkspace ")
        );
        let body = "data: {\"type\":\"server.connected\",\"properties\":{}}\n\ndata: {\"type\":\"session.status\",\"properties\":{\"sessionID\":\"ses_1\",\"status\":{\"type\":\"busy\"}}}\n\n";
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
    });
    let client = test_client(address);
    pollster::block_on(async {
        let mut events = client.subscribe_events().await.unwrap();
        assert!(matches!(
            events.next().await.unwrap().unwrap(),
            Event::ServerConnected
        ));
        assert!(matches!(
            events.next().await.unwrap().unwrap(),
            Event::SessionStatus { session_id, .. } if session_id == "ses_1"
        ));
    });
    server.join().unwrap();
}

#[test]
fn fetches_typed_directory_catalogs() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 2048];
            let length = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..length]);
            let body = if request.starts_with("GET /agent?directory=%2Fworkspace ") {
                r#"[{"name":"build","description":"default","mode":"primary"},{"name":"explore","mode":"subagent","hidden":null}]"#
            } else if request.starts_with("GET /provider?directory=%2Fworkspace ") {
                r#"{"all":[{"id":"openai","name":"OpenAI","models":{"gpt-test":{"id":"gpt-test","providerID":"openai","name":"GPT Test","status":"active","variants":{"high":{"reasoningEffort":"high"}}}}}],"default":{"openai":"gpt-test"},"connected":["openai"]}"#
            } else {
                panic!("unexpected request: {request}");
            };
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
    });
    let catalog = pollster::block_on(test_client(address).catalog()).unwrap();
    assert_eq!(catalog.agents[0].name, "build");
    assert_eq!(catalog.agents[1].hidden, None);
    assert_eq!(catalog.providers.connected, ["openai"]);
    let model = &catalog.providers.all[0].models["gpt-test"];
    assert_eq!(model.provider_id, "openai");
    assert!(model.variants.contains_key("high"));
    server.join().unwrap();
}

fn test_client(address: std::net::SocketAddr) -> Client {
    Client::new(
        &format!("http://{address}"),
        Some("/workspace".into()),
        None,
        None,
    )
    .unwrap()
}
