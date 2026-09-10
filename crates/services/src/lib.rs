#![no_std]
pub mod health;
pub mod storage;

use core::fmt::Display;

pub use trouble_host;
use trouble_host::types::gatt_traits::{AsGatt, FromGatt, FromGattError};

const fn uuid_to_ble_bytes(uuid: &uuid::Uuid) -> [u8; 16] {
    let b = *uuid.as_bytes();
    [
        b[15], b[14], b[13], b[12], b[11], b[10], b[9], b[8], b[7], b[6], b[5], b[4], b[3], b[2],
        b[1], b[0],
    ]
}

pub trait Foo {
    type ReadData: FromGatt + Display;
    type NotificationData: FromGatt + Display;
    type Id;
    type WriteData;

    fn deserialize_read_response(&self, data: &[u8]) -> Result<Self::ReadData, FromGattError>;
    fn deserialize_notification_response(&self, data: &[u8]) -> Result<Self::NotificationData, FromGattError>;
    fn serialize_write_data(&self, data: Self::WriteData) -> impl AsGatt;
    fn id(&self) -> Self::Id;
}
