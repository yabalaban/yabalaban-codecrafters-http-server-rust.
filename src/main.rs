use lazy_static::lazy_static;

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::prelude::*;
use std::path::Path;
use std::str::FromStr;
use std::sync::Mutex;

use std::thread;
use std::net::{SocketAddr, TcpListener, TcpStream, IpAddr, Ipv4Addr};

const CRLF: &str = "\r\n";

/*  Disclaimer: bad-quality code is expected, I had no intention to do things right but to hack around with Rust. */

// Hacky workaround for global context

#[derive(Clone)]
struct ExecutionContext {
    directory: Option<String>
}

lazy_static! {
    static ref GLOBAL_CONTEXT: Mutex<ExecutionContext> = Mutex::new(ExecutionContext { directory: None });
}

fn get_context() -> ExecutionContext {
    GLOBAL_CONTEXT.lock().unwrap().clone()
}

fn set_context(context: ExecutionContext) {
    *GLOBAL_CONTEXT.lock().unwrap() = context
}

// Some HTTP internals and fancy structs
#[derive(PartialEq)]
enum HTTPMethod {
    GET,
    POST,
}

impl FromStr for HTTPMethod {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GET" => Ok(HTTPMethod::GET),
            "POST" => Ok(HTTPMethod::POST),
            _ => Err(()),
        }
    }
}

struct HTTPRequest {
    method: HTTPMethod,
    path: String,
    headers: HashMap<String, String>,
    body: String,
}

fn parse_request_str(raw: String) -> Result<HTTPRequest, ()> {
    let lines = raw.split(CRLF)
        .collect::<Vec<&str>>();
    assert!(lines.len() > 1);
    let start_line = lines[0].split(' ')
        .collect::<Vec<&str>>();
    assert!(start_line.len() == 3);

    let mut headers = HashMap::new();
    let mut body = String::new();
    let mut append_body = false;

    if lines.len() > 2 {
        for line in lines[2..].into_iter() {
            if line.len() == 0 { append_body = true; continue; }
            if append_body {
                body.push_str(line.to_owned().trim_end_matches(char::from(0)));
            } else {
                let header = line.split(": ")
                    .collect::<Vec<&str>>();
                if header.len() == 2 {
                    headers.insert(header[0].to_string(), header[1].to_string());
                }
            }
        }
    }

    let method = HTTPMethod::from_str(start_line[0]).unwrap();
    let request = HTTPRequest {
        method: method,
        path: start_line[1].to_string(),
        headers: headers,
        body: body,
    };
    Ok(request)
}

enum HTTPResponseStatusCode {
    Ok,
    Created,
    NotFound,
}

impl ToString for HTTPResponseStatusCode {
    fn to_string(&self) -> String {
        match &self {
            HTTPResponseStatusCode::Ok => "200 OK".to_owned(),
            HTTPResponseStatusCode::Created => "201 Created".to_owned(),
            HTTPResponseStatusCode::NotFound => "404 Not Found".to_owned(),
        }
    }
}

enum HTTPContentType {
    PlainText,
    ApplicationOctetStream,
}

impl ToString for HTTPContentType {
    fn to_string(&self) -> String {
        match &self {
            HTTPContentType::PlainText => "Content-Type: text/plain".to_owned(),
            HTTPContentType::ApplicationOctetStream => "Content-Type: application/octet-stream".to_owned(),
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

// Input stream handling

#[derive(Clone, Copy)]
struct RequestHandler {
    predicate: fn(&HTTPRequest) -> bool,
    handle: fn(&HTTPRequest) -> HTTPResponse,
}

fn handle_request(request: HTTPRequest, handlers: &[RequestHandler]) -> HTTPResponse {
    let handler = handlers.into_iter().find(|h| (h.predicate)(&request));
    match handler {
        Some(h) => (h.handle)(&request),
        None => HTTPResponse { status_code: HTTPResponseStatusCode::NotFound, payload: None },
    }
}

fn handle_stream(mut stream: TcpStream, handlers: &[RequestHandler]) -> std::io::Result<usize> {
    let mut buffer = [0; 2048];
    stream.read(&mut buffer)?;
    let request_str = String::from_utf8_lossy(&buffer).to_string();
    let request = parse_request_str(request_str).unwrap();
    let response = handle_request(request, handlers);
    stream.write(response.to_string().as_bytes())
}
fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let localhost = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4221);
    let listener = TcpListener::bind(localhost)?;

    if args.len() > 1 && args[1] == "--directory" {
        let mut context = get_context();
        context.directory = Some(args[2].clone());
        set_context(context);
    } 

    let echo = RequestHandler {
        predicate: |request| request.path.starts_with("/echo"),
        handle: |request| {
            let components = request.path.split('/')
                .collect::<Vec<&str>>();
            HTTPResponse { 
                status_code: HTTPResponseStatusCode::Ok, 
                payload: Some(HTTPResponsePayload {
                    content_type: HTTPContentType::PlainText,
                    payload: components[2..].join("/").to_owned(),
                }) 
            }
        }
    };

    let user_agent = RequestHandler {
        predicate: |request| request.path.starts_with("/user-agent"),
        handle: |request| {
            HTTPResponse { 
                status_code: HTTPResponseStatusCode::Ok, 
                payload: Some(HTTPResponsePayload {
                    content_type: HTTPContentType::PlainText,
                    payload: request.headers.get("User-Agent").unwrap().to_owned(),
                }) 
            }
        }
    };

    let main = RequestHandler {
        predicate: |request| request.path == "/",
        handle: |_request| HTTPResponse { status_code: HTTPResponseStatusCode::Ok, payload: None },
    };

    let get_file = RequestHandler {
        predicate: |request| request.method == HTTPMethod::GET && request.path.starts_with("/files"),
        handle: |request| { 
            let components = request.path.split('/').collect::<Vec<&str>>();
            let filename = components[2].to_owned();
            let context = get_context();
            let filepath = format!("{}{}", context.directory.unwrap(), filename);
            if Path::new(filepath.as_str()).exists() { 
                HTTPResponse { 
                    status_code: HTTPResponseStatusCode::Ok, 
                    payload: Some(HTTPResponsePayload {
                        content_type: HTTPContentType::ApplicationOctetStream,
                        payload: fs::read_to_string(filepath).unwrap().parse().unwrap(),
                    })
                }
            } else {
                HTTPResponse { status_code: HTTPResponseStatusCode::NotFound, payload: None }
            }
        }
    };

    let post_file = RequestHandler {
        predicate: |request| request.method == HTTPMethod::POST && request.path.starts_with("/files"),
        handle: |request| { 
            let components = request.path.split('/').collect::<Vec<&str>>();
            let filename = components[2].to_owned();
            let context = get_context();
            let filepath = format!("{}{}", context.directory.unwrap(), filename);
            fs::write(filepath, request.body.clone()).expect("should be fine"); 
            HTTPResponse { 
                status_code: HTTPResponseStatusCode::Created, 
                payload: None
            }
        }
    };

    let handlers = [
        echo,
        user_agent,
        get_file,
        post_file,
        main,
    ];

    for stream in listener.incoming() {
        match stream {
            Ok(stream_) => _ = thread::spawn(move || {
                _ = handle_stream(stream_, &handlers);
            }),
            Err(e) => println!("{}", e)   
        }
    }
    Ok(())
}
