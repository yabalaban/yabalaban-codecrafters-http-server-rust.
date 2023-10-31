use std::collections::HashMap;
use std::io::prelude::*;
use std::thread;
use std::net::{SocketAddr, TcpListener, TcpStream, IpAddr, Ipv4Addr};

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
    if lines.len() > 2 {
        for line in lines[2..].into_iter() {
            if line.len() == 0 { continue; }
            let header = line.split(": ")
                .collect::<Vec<&str>>();
            if header.len() == 2 {
                headers.insert(header[0].to_string(), header[1].to_string());
            }
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

enum HTTPResponseStatusCode {
    Ok,
    NotFound,
}

impl ToString for HTTPResponseStatusCode {
    fn to_string(&self) -> String {
        match &self {
            HTTPResponseStatusCode::Ok => "200 OK".to_owned(),
            HTTPResponseStatusCode::NotFound => "404 Not Found".to_owned(),
        }
    }
}

enum HTTPContentType {
    PlainText
}

impl ToString for HTTPContentType {
    fn to_string(&self) -> String {
        match &self {
            HTTPContentType::PlainText => "Content-Type: text/plain".to_owned(),
        }
    }
}

struct HTTPResponsePayload { 
    content_type: HTTPContentType,
    payload: String,
}

impl ToString for HTTPResponsePayload {
    fn to_string(&self) -> String {
        let content_length = format!("Content-Length: {}", self.payload.len());
        format!("{}\r\n{}\r\n\r\n{}", self.content_type.to_string(), content_length, self.payload)
    }
}

struct HTTPResponse { 
    status_code: HTTPResponseStatusCode,
    payload: Option<HTTPResponsePayload>,
}

impl ToString for HTTPResponse {
    fn to_string(&self) -> String {
        let status = format!("HTTP/1.1 {}", self.status_code.to_string());
        match &self.payload {
            Some(val) => format!("{}\r\n{}", status, val.to_string()),
            None => format!("{}\r\n\r\n", status),
        }
    }
}

fn handle_request(request: HTTPRequest) -> HTTPResponse {
    match request.path.as_str() {
        path if path.starts_with("/echo") => {
            let components = path.split('/')
                .collect::<Vec<&str>>();
            HTTPResponse { 
                status_code: HTTPResponseStatusCode::Ok, 
                payload: Some(HTTPResponsePayload {
                    content_type: HTTPContentType::PlainText,
                    payload: components[2..].join("/").to_owned(),
                }) 
            }
        },
        path if path.starts_with("/user-agent") => {
            HTTPResponse { 
                status_code: HTTPResponseStatusCode::Ok, 
                payload: Some(HTTPResponsePayload {
                    content_type: HTTPContentType::PlainText,
                    payload: request.headers.get("User-Agent").unwrap().to_owned(),
                }) 
            }
        },
        "/" => HTTPResponse { status_code: HTTPResponseStatusCode::Ok, payload: None },
        _ => HTTPResponse { status_code: HTTPResponseStatusCode::NotFound, payload: None },
    }
}

fn handle_stream(mut stream: TcpStream) -> std::io::Result<usize> {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer)?;
    let request_str = String::from_utf8_lossy(&buffer).to_string();
    let request = parse_request_str(request_str).unwrap();
    let response = handle_request(request);
    stream.write(response.to_string().as_bytes())
}

fn main() -> std::io::Result<()> {
    let localhost = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4221);
    let listener = TcpListener::bind(localhost)?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream_) => _ = thread::spawn(|| {
                _ = handle_stream(stream_);
            }),
            Err(e) => println!("{}", e)   
        }
    }
    Ok(())
}
