use super::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

/// Spawn a tiny single-threaded HTTP/1.1 server on an ephemeral localhost port
/// and return its base URL (`http://127.0.0.1:<port>`). Each accepted
/// connection is read just enough to recover the request target path, then
/// handed to `responder`, which writes a raw HTTP response into the stream.
/// Every response advertises `Connection: close`, so the client opens a fresh
/// connection per request (including each hop of a redirect). The server
/// thread runs until the process exits; tests are short-lived, so this is fine.
fn spawn_server<F>(responder: F) -> String
where
    F: Fn(&str, &mut TcpStream) + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind failed");
    let addr = listener.local_addr().expect("local_addr failed");
    std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let path = read_request_target(&mut stream);
            responder(&path, &mut stream);
        }
    });
    format!("http://{addr}")
}

/// Best-effort read of the request line, returning the request target (the
/// path component, e.g. `/json`). Loops until the first CRLF arrives so a
/// split first packet is handled, and falls back to `/` if nothing useful
/// arrives.
fn read_request_target(stream: &mut TcpStream) -> String {
    let mut buf = [0u8; 4096];
    let mut filled = 0;
    while filled < buf.len() {
        match stream.read(&mut buf[filled..]) {
            Ok(0) | Err(_) => break,
            Ok(n) => filled += n,
        }
        if buf[..filled]
            .windows(2)
            .any(|w| w[0] == b'\r' && w[1] == b'\n')
        {
            break;
        }
    }
    String::from_utf8_lossy(&buf[..filled])
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/")
        .to_string()
}

/// Write a minimal HTTP/1.1 response. `extra_headers` is an optional header
/// block without a trailing CRLF (e.g. `"Location: /id/237"`); it is omitted
/// entirely when empty.
fn respond(stream: &mut TcpStream, status: u16, reason: &str, extra_headers: &str, body: &[u8]) {
    let headers = if extra_headers.is_empty() {
        String::new()
    } else {
        format!("{extra_headers}\r\n")
    };
    let _ = write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nConnection: close\r\n{headers}Content-Length: {}\r\n\r\n",
        body.len(),
    );
    let _ = stream.write_all(body);
}

#[tokio::test]
async fn shared_client_builds_ok() {
    assert!(
        shared_client().is_ok(),
        "shared_client should build a reqwest client",
    );
}

#[tokio::test]
async fn get_text_returns_body_on_200() {
    let base = spawn_server(|_path, stream| {
        respond(stream, 200, "OK", "", b"hello world");
    });
    let client = shared_client().expect("client");
    let body = get_text(&client, &format!("{base}/text"))
        .await
        .expect("200 should succeed");
    assert_eq!(body, "hello world");
}

#[tokio::test]
async fn get_text_errors_on_404() {
    let base = spawn_server(|_path, stream| {
        respond(stream, 404, "Not Found", "", b"nope");
    });
    let client = shared_client().expect("client");
    let err = get_text(&client, &format!("{base}/missing"))
        .await
        .expect_err("404 should error");
    let msg = err.to_string();
    assert!(
        msg.contains("HTTP 404"),
        "error should mention the status: {msg}",
    );
}

#[tokio::test]
async fn get_json_parses_body_on_200() {
    let base = spawn_server(|_path, stream| {
        respond(stream, 200, "OK", "", br#"{"hello":"world","n":42}"#);
    });
    let client = shared_client().expect("client");
    let json = get_json(&client, &format!("{base}/json"))
        .await
        .expect("200 + valid JSON should succeed");
    assert_eq!(json["hello"].as_str(), Some("world"));
    assert_eq!(json["n"].as_i64(), Some(42));
}

#[tokio::test]
async fn get_json_errors_on_invalid_body() {
    let base = spawn_server(|_path, stream| {
        respond(stream, 200, "OK", "", b"not json");
    });
    let client = shared_client().expect("client");
    let err = get_json(&client, &format!("{base}/json"))
        .await
        .expect_err("invalid JSON should error");
    let msg = err.to_string();
    assert!(
        msg.contains("failed to parse JSON"),
        "error should mention JSON parse failure: {msg}",
    );
}

#[tokio::test]
async fn get_bytes_redirected_returns_body_and_final_url_on_200() {
    let base = spawn_server(|path, stream| {
        if path == "/img" {
            respond(stream, 200, "OK", "", &[1, 2, 3, 4, 5]);
        } else {
            respond(stream, 404, "Not Found", "", b"nope");
        }
    });
    let client = shared_client().expect("client");
    let (bytes, final_url) = get_bytes_redirected(&client, &format!("{base}/img"))
        .await
        .expect("200 should succeed");
    assert_eq!(bytes, vec![1u8, 2, 3, 4, 5]);
    assert_eq!(final_url, format!("{base}/img"));
}

#[tokio::test]
async fn get_bytes_redirected_follows_redirect_to_final_url() {
    let base = spawn_server(|path, stream| {
        if path == "/seed" {
            respond(stream, 302, "Found", "Location: /id/237", b"");
        } else if path == "/id/237" {
            respond(stream, 200, "OK", "", &[9, 9, 9]);
        } else {
            respond(stream, 404, "Not Found", "", b"nope");
        }
    });
    let client = shared_client().expect("client");
    let (bytes, final_url) = get_bytes_redirected(&client, &format!("{base}/seed"))
        .await
        .expect("redirect then 200 should succeed");
    assert_eq!(bytes, vec![9u8, 9, 9]);
    assert_eq!(final_url, format!("{base}/id/237"));
}
