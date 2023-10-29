// Uncomment this block to pass the first stage
use std::net::{SocketAddr, TcpListener, TcpStream, IpAddr, Ipv4Addr};

fn handle_stream(_stream: TcpStream) {
    println!("accepted new connection");
}

fn main() -> std::io::Result<()> {
    let localhost = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4221);
    let listener = TcpListener::bind(localhost)?;
    for stream in listener.incoming() {
        handle_stream(stream?);
    }
    Ok(())
}
