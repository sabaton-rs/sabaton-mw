/// Topics for use in Service Discovery
/// We use DDS to locate the IP address and port of services
use cyclonedds_rs::*;
use std::net::IpAddr;
use serde_derive::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Topic)]
pub struct ServiceInfo {
    pub node: String,
    pub major_version: u8,
    pub minor_version: u32,
    #[topic_key]
    pub instance : u16,
    pub socket_address: std::net::SocketAddr,
    pub transport : Transport,
   
}

#[derive(Serialize, Deserialize)]
pub enum Transport {
    Udp,
    Tcp,
}