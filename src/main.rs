use std::collections::HashMap;
use std::io::prelude::*;
use std::net::{SocketAddr, TcpListener, TcpStream, IpAddr, Ipv4Addr};

use nom::AsBytes;

const CRLF: &str = "\r\n";

enum HTTPMethod {
    GET
}

struct HTTPRequest {
    method: HTTPMethod,
    path: String,
    version: String,
    headers: HashMap<String, String>,
}

fn parse_request_str(raw: String) -> Result<HTTPRequest, ()> {
    let lines = raw.split(CRLF)
        .collect::<Vec<&str>>();
    assert!(lines.len() > 1);
    let start_line = lines[0].split(' ')
        .collect::<Vec<&str>>();
    assert!(start_line.len() == 3);

    let mut headers = HashMap::new();
    if start_line.len() > 2 {
        for line in start_line[2..].into_iter() {
            let header = line.split(": ")
                .collect::<Vec<&str>>();
            assert!(header.len() == 2);
            headers.insert(header[0].to_string(), header[1].to_string());
        }
    }
    let request = HTTPRequest {
        method: HTTPMethod::GET,
        path: start_line[1].to_string(),
        version: start_line[2].to_string(),
        headers: headers
    };
    Ok(request)
}

fn write_response_code(mut stream: TcpStream, status: &str) -> std::io::Result<usize> {
    let response = format!("HTTP/1.1 {status} {CRLF}{CRLF}");
    stream.write(response.as_bytes())
}

fn handle_stream(mut stream: TcpStream) -> std::io::Result<usize> {
    // let mut request_str = String::new(); 
    // stream.read_to_string(&mut request_str)?;
    let mut buffer = Vec::new();
    stream.read_to_end(&mut buffer)?;
    let request_str = String::from_utf8_lossy(buffer.as_bytes()).to_string();
    let request = parse_request_str(request_str).unwrap();
    match request.path.as_str() {
        "/" => write_response_code(stream, "200 OK"),
        _ => write_response_code(stream, "404 Not Found"),
    }
}

fn main() -> std::io::Result<()> {
    let localhost = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4221);
    let listener = TcpListener::bind(localhost)?;
    for stream in listener.incoming() {
        _ = handle_stream(stream?);
    }
    Ok(())
}
