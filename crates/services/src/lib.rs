#![no_std]
pub mod health;
pub mod storage;

use core::fmt::Display;

use crate::{
    health::{
        HEALTH_PING_CHAR_UUID, HEALTH_STATUS_CHAR_UUID, HealthServicePingHandler,
        HealthServiceStatusHandler, Pong, Status,
    },
    storage::{STORAGE_DATA_CHAR_UUID, StorageServiceDataHandler},
};
pub use trouble_host;
use trouble_host::types::gatt_traits::{AsGatt, FromGatt, FromGattError};
use uuid::Uuid;

const fn uuid_to_ble_bytes(uuid: &uuid::Uuid) -> [u8; 16] {
    let b = *uuid.as_bytes();
    [
        b[15], b[14], b[13], b[12], b[11], b[10], b[9], b[8], b[7], b[6], b[5], b[4], b[3], b[2],
        b[1], b[0],
    ]
}

trait IotCharacteristicReadHandler {
    type ReadData: FromGatt + Display;
    fn deserialize(data: &[u8]) -> Result<Self::ReadData, FromGattError>;
}

trait IotCharacteristicWriteHandler {
    type WriteData: AsGatt;
    fn serialize(data: &[u8]) -> Result<Self::WriteData, WriteError>;
}

trait IotCharacteristicNotificationHandler {
    type NotificationData: FromGatt + Display;
    fn deserialize(data: &[u8]) -> Result<Self::NotificationData, FromGattError>;
}

#[derive(Debug, Clone)]
pub enum ReadHandler {
    Status,
    Data,
    Other,
}

pub enum ReadResponse {
    Status(Status),
    Data(u8),
    Other,
}

impl ReadHandler {
    pub fn new(uuid: Uuid) -> Self {
        if uuid == HEALTH_STATUS_CHAR_UUID {
            Self::Status
        } else if uuid == STORAGE_DATA_CHAR_UUID {
            Self::Data
        } else {
            Self::Other
        }
    }

    pub fn deserialize(&self, data: &[u8]) -> Result<ReadResponse, FromGattError> {
        match self {
            Self::Status => {
                let status =
                    <HealthServiceStatusHandler as IotCharacteristicReadHandler>::deserialize(
                        data,
                    )?;
                Ok(ReadResponse::Status(status))
            }
            Self::Data => {
                let data =
                    <StorageServiceDataHandler as IotCharacteristicReadHandler>::deserialize(
                        data,
                    )?;
                Ok(ReadResponse::Data(data))
            }
            Self::Other => Ok(ReadResponse::Other),
        }
    }
}

#[derive(Debug, Clone)]
pub enum WriteHandler {
    Data,
    Other,
}

pub enum WriteResponse {
    Data(u8),
    Other,
}

pub enum WriteError {}

impl WriteHandler {
    pub fn new(uuid: Uuid) -> Self {
        if uuid == STORAGE_DATA_CHAR_UUID {
            Self::Data
        } else {
            Self::Other
        }
    }

    pub fn serialize(&self, data: &[u8]) -> Result<WriteResponse, WriteError> {
        match self {
            Self::Data => {
                let bar =
                    <StorageServiceDataHandler as IotCharacteristicWriteHandler>::serialize(
                        data,
                    )?;
                Ok(WriteResponse::Data(bar))
            }
            Self::Other => Ok(WriteResponse::Other),
        }
    }
}

#[derive(Debug, Clone)]
pub enum NotificationHandler {
    Ping,
    Other,
}

pub enum NotificationResponse {
    Ping(Pong),
    Other,
}

impl NotificationHandler {
    pub fn new(uuid: Uuid) -> Self {
        if uuid == HEALTH_PING_CHAR_UUID {
            Self::Ping
        } else {
            Self::Other
        }
    }

    pub fn deserialize(&self, data: &[u8]) -> Result<NotificationResponse, FromGattError> {
        match self {
            Self::Ping => {
                let pong = <HealthServicePingHandler as IotCharacteristicNotificationHandler>::deserialize(data)?;
                Ok(NotificationResponse::Ping(pong))
            }
            Self::Other => Ok(NotificationResponse::Other),
        }
    }
}
