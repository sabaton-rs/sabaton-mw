/*
    Copyright (C) Sabaton Systems LLP - All Rights Reserved
    Sojan James <sojan.james@gmail.com>, 2021

    SPDX-License-Identifier: Apache-2.0 OR LicenseRef-sabaton-commercial
*/

use std::time::Duration;
use thiserror::Error;

#[derive(Clone)]
pub enum QosReliability {
    BestEffort(Duration),
    Reliable(Duration),
}

impl Default for QosReliability {
    fn default() -> Self {
        Self::Reliable(Duration::from_millis(100))
    }
}

#[derive(Clone)]
pub enum QosDurability {
    Volatile,        // Receive only new data
    TransientLocal,  // Receive valid data even if old
}

impl Default for QosDurability {
    fn default() -> Self {
        Self::Volatile
    }
}

#[derive(Clone)]
pub enum QosHistory {
    KeepAll,
    KeepLast(usize),
}

impl Default for QosHistory {
    fn default() -> Self {
        Self::KeepLast(1)
    }
}
pub trait Qos : Default {
    fn set_reliability(&mut self, reliability: QosReliability) -> Result<(),QosError>;
    fn set_durability(&mut self, durability: QosDurability) -> Result<(),QosError>;
    fn set_history(&mut self, history: QosHistory) -> Result<(),QosError>;
}

pub trait QosCreate : Qos {
    fn create() ->  Self;
}

#[derive(Error, Debug)]
pub enum QosError {
    #[error("Internal DDS error")]
    InternalError,
}
