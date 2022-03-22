use std::net::SocketAddr;


pub fn get_bind_address() -> SocketAddr {
    let socket_addr : SocketAddr = "127.0.0.1:0".parse().unwrap();
    socket_addr
}