/*
    Copyright (C) Sabaton Systems LLP - All Rights Reserved
    Sojan James <sojan.james@gmail.com>, 2021

    SPDX-License-Identifier: Apache-2.0 OR LicenseRef-sabaton-commercial
*/

use std::net::SocketAddr;


pub fn get_bind_address() -> SocketAddr {
    let socket_addr : SocketAddr = "127.0.0.1:0".parse().unwrap();
    socket_addr
}


#[derive(Clone)]
pub enum LogLevel {
    Verbose,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
    Off,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Verbose => "verbose",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
            LogLevel::Fatal => "fatal",
            LogLevel::Off => "off",
        }
    }
}