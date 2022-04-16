/*
    Copyright (C) Sabaton Systems LLP - All Rights Reserved
    Sojan James <sojan.james@gmail.com>, 2021

    SPDX-License-Identifier: GPL-3.0-only OR LicenseRef-sabaton-commercial
*/

/// Topics for use in Service Discovery
/// We use DDS to locate the IP address and port of services
use cyclonedds_rs::*;

use serde_derive::{Serialize, Deserialize};
use tracing::{debug};

#[derive(Serialize, Deserialize, Topic, Debug)]
pub struct ServiceInfo {
    pub node: String,
    #[topic_key]
    pub major_version: u8,
    #[topic_key]
    pub minor_version: u32,
    #[topic_key]
    pub instance_id : u16,
    pub socket_address: std::net::SocketAddr,
    pub transport : Transport,
    pub service_id : u16,
   
}

#[derive(Serialize, Deserialize, Debug, PartialEq,Clone)]
pub enum Transport {
    Udp,
    Tcp,
}

/// Create a topic for a service name.
pub fn service_name_to_topic_name(service_name: &str) -> String {
    let replaced_dots = service_name.replace(".", "/");
    let topic_name = "/service_discovery/default/".to_owned() + replaced_dots.as_str();

    debug!("SD topic name for {} is {}", service_name, &topic_name);
    topic_name
}
