/*
    Copyright (C) Sabaton Systems LLP - All Rights Reserved
    Sojan James <sojan.james@gmail.com>, 2021

    SPDX-License-Identifier: GPL-3.0-only OR LicenseRef-sabaton-commercial
*/

use std::net::SocketAddr;


pub fn get_bind_address() -> SocketAddr {
    let socket_addr : SocketAddr = "127.0.0.1:0".parse().unwrap();
    socket_addr
}