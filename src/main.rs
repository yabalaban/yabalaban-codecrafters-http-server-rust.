use std::io::prelude::*;
use std::net::{SocketAddr, TcpListener, TcpStream, IpAddr, Ipv4Addr};

fn handle_stream(mut stream: TcpStream) -> std::io::Result<usize> {
    stream.read(&mut [0; 128])?;
    stream.write(b"HTTP/1.1 200 OK\r\n\r\n")
}

fn main() -> std::io::Result<()> {
    let localhost = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4221);
    let listener = TcpListener::bind(localhost)?;
    for stream in listener.incoming() {
        _ = handle_stream(stream?);
    }
    Ok(())
}
