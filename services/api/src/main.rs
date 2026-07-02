use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

fn main() -> std::io::Result<()> {
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let address = format!("{host}:{port}");
    let listener = TcpListener::bind(&address)?;

    println!("drive-clone-api listening on {address}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = handle_connection(stream) {
                    eprintln!("request failed: {error}");
                }
            }
            Err(error) => eprintln!("connection failed: {error}"),
        }
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;

    let mut buffer = [0_u8; 2048];
    let bytes_read = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let request_line = request.lines().next().unwrap_or_default();
    let response = response_for_request_line(request_line);

    stream.write_all(response.as_bytes())?;
    stream.flush()
}

fn response_for_request_line(request_line: &str) -> String {
    let path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/");

    match path {
        "/" => json_response(
            "200 OK",
            r#"{"service":"drive-clone-api","message":"Google Drive clone API scaffold"}"#,
        ),
        "/health" => json_response(
            "200 OK",
            r#"{"status":"ok","service":"drive-clone-api"}"#,
        ),
        _ => json_response(
            "404 Not Found",
            r#"{"error":"not_found"}"#,
        ),
    }
}

fn json_response(status: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

#[cfg(test)]
mod tests {
    use super::response_for_request_line;

    #[test]
    fn health_returns_ok() {
        let response = response_for_request_line("GET /health HTTP/1.1");

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains(r#""status":"ok""#));
        assert!(response.contains("Content-Type: application/json"));
    }

    #[test]
    fn unknown_path_returns_404() {
        let response = response_for_request_line("GET /missing HTTP/1.1");

        assert!(response.starts_with("HTTP/1.1 404 Not Found"));
        assert!(response.contains(r#""error":"not_found""#));
    }
}
