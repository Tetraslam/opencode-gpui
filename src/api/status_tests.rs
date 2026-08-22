use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

use super::{Client, McpStatus};

#[test]
fn fetches_typed_directory_status_snapshot() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        for _ in 0..4 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 2048];
            let length = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..length]);
            assert!(request.contains("?directory=%2Fworkspace "));
            let body = if request.starts_with("GET /mcp?") {
                r#"{"github":{"status":"connected"},"future":{"status":"warming","error":"not ready"}}"#
            } else if request.starts_with("GET /lsp?") {
                r#"[{"id":"rust-analyzer","name":"Rust","root":"/workspace","status":"connected"}]"#
            } else if request.starts_with("GET /formatter?") {
                r#"[{"name":"rustfmt","enabled":true},{"name":"prettier","enabled":false}]"#
            } else if request.starts_with("GET /config?") {
                r#"{"plugin":["pkg@1.2.3",["file:///workspace/plugin.ts",{"option":true}],{"future":true}]}"#
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
    let client = Client::new(
        &format!("http://{address}"),
        Some("/workspace".into()),
        None,
        None,
    )
    .unwrap();
    let snapshot = pollster::block_on(client.status_snapshot()).unwrap();
    assert_eq!(snapshot.lsp[0].id, "rust-analyzer");
    assert!(snapshot.formatters[0].enabled);
    assert_eq!(snapshot.config.plugins.len(), 3);
    assert_eq!(
        snapshot.mcp["future"],
        McpStatus::Unknown {
            status: "warming".into(),
            detail: Some("not ready".into())
        }
    );
    server.join().unwrap();
}

#[test]
fn null_plugin_config_is_an_empty_list() {
    let config: super::StatusConfig = serde_json::from_str(r#"{"plugin":null}"#).unwrap();
    assert!(config.plugins.is_empty());
}

#[test]
fn mcp_connection_mutations_match_scoped_paths() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        for expected in [
            "POST /mcp/github%20server/connect?directory=%2Fworkspace ",
            "POST /mcp/github%20server/disconnect?directory=%2Fworkspace ",
        ] {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 2048];
            let length = stream.read(&mut request).unwrap();
            assert!(String::from_utf8_lossy(&request[..length]).starts_with(expected));
            let body = "true";
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
    });
    let client = Client::new(
        &format!("http://{address}"),
        Some("/workspace".into()),
        None,
        None,
    )
    .unwrap();
    assert!(pollster::block_on(client.connect_mcp("github server")).unwrap());
    assert!(pollster::block_on(client.disconnect_mcp("github server")).unwrap());
    server.join().unwrap();
}
