/*
 * port.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

use std::{
    error::Error,
    net::{SocketAddr, TcpListener},
};

pub struct RandomUserPort(pub u16);

impl RandomUserPort {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let res = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))?;

        Ok(Self(res.local_addr()?.port()))
    }
}
