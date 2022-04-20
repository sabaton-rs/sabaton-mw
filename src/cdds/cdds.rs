/*
    Copyright (C) Sabaton Systems LLP - All Rights Reserved
    Sojan James <sojan.james@gmail.com>, 2021

    SPDX-License-Identifier: GPL-3.0-only OR LicenseRef-sabaton-commercial
*/

use cyclonedds_rs::{DdsQos, dds_reliability_kind};

use crate::qos::{QosReliability,Qos, QosError, QosDurability, QosHistory, QosCreate};


pub struct CddsQos(DdsQos);

impl Default for CddsQos {
    fn default() -> Self {
        let mut qos = CddsQos::create();
        qos.set_durability(QosDurability::default()).expect("qos");
        qos.set_reliability(QosReliability::default()).expect("qos");
        qos.set_history(QosHistory::default()).expect("qos");
        qos
    }
}

impl QosCreate for CddsQos {
    fn create() ->  Self {
        let qos = DdsQos::create().expect("unable to create QOS");
         CddsQos(qos)
    }
}


impl Qos for CddsQos {

    fn set_reliability(&mut self, reliability: QosReliability) -> Result<(),crate::qos::QosError> {
        match reliability {
            QosReliability::BestEffort(max_blocking_time) => self.0.set_reliability(dds_reliability_kind::DDS_RELIABILITY_BEST_EFFORT, max_blocking_time),
            QosReliability::Reliable(max_blocking_time) => self.0.set_reliability(dds_reliability_kind::DDS_RELIABILITY_RELIABLE, max_blocking_time),
        };
        Ok(())
    }

    fn set_durability(&mut self, durability: crate::qos::QosDurability) -> Result<(),crate::qos::QosError> {
        match durability {
            crate::qos::QosDurability::Volatile => self.0.set_durability(cyclonedds_rs::dds_durability_kind::DDS_DURABILITY_VOLATILE),
            crate::qos::QosDurability::TransientLocal => self.0.set_durability(cyclonedds_rs::dds_durability_kind::DDS_DURABILITY_TRANSIENT_LOCAL),
        };
        Ok(())
    }

    fn set_history(&mut self, history: crate::qos::QosHistory) -> Result<(),crate::qos::QosError> {
        match history {
            crate::qos::QosHistory::KeepAll => self.0.set_history(cyclonedds_rs::dds_history_kind::DDS_HISTORY_KEEP_ALL, 0),
            crate::qos::QosHistory::KeepLast(depth) => self.0.set_history(cyclonedds_rs::dds_history_kind::DDS_HISTORY_KEEP_LAST, depth as i32),
        };
        Ok(())
    }
}

impl Into<DdsQos> for CddsQos {
    fn into(self) -> DdsQos {
        self.0
    }
}