use crate::Foo;

use super::uuid_to_ble_bytes;
use log::info;
use trouble_host::{
    prelude::gatt_service,
    types::gatt_traits::{AsGatt, FromGatt, FromGattError},
};
use util::WriteData;
use uuid::Uuid;

const STORAGE_SERVICE_UUID: Uuid = Uuid::from_u128(0xc7d9a5b06c1a4b2c9b3a3d45e6a20000);

pub const STORAGE_DATA_CHAR_UUID: Uuid = Uuid::from_u128(0xc7d9a5b06c1a4b2c9b3a3d45e6a20001);

pub const STORAGE_DATA_DESCRIPTOR_UUID: Uuid = Uuid::from_u128(0xc7d9a5b06c1a4b2c9b3a3d45e6a21001);

#[gatt_service(uuid = uuid_to_ble_bytes(&STORAGE_SERVICE_UUID))]
pub struct StorageService {
    #[descriptor(uuid = uuid_to_ble_bytes(&STORAGE_DATA_DESCRIPTOR_UUID), read, value = StorageServiceDataDescriptor, type = StorageServiceDataDescriptor)]
    #[characteristic(uuid = uuid_to_ble_bytes(&STORAGE_DATA_CHAR_UUID), read, write, value=42)]
    pub data: u8,
}

impl StorageService {
    pub fn data_handle(&self) -> u16 {
        self.data.handle
    }

    pub async fn process_write_data_request(&self, _write_data: WriteData) {
        info!("[storage_service] handling writing data");
    }
}

#[derive(Debug, Clone)]
pub struct StorageServiceDataDescriptor;

impl AsGatt for StorageServiceDataDescriptor {
    const MIN_SIZE: usize = core::mem::size_of::<u8>();
    const MAX_SIZE: usize = core::mem::size_of::<u8>();

    fn as_gatt(&self) -> &[u8] {
        &[]
    }
}

impl FromGatt for StorageServiceDataDescriptor {
    fn from_gatt(_data: &[u8]) -> Result<Self, FromGattError> {
        Ok(Self)
    }
}

impl Foo for StorageServiceDataDescriptor {
    type ReadData = u8;
    type Id = Uuid;
    type WriteData = [u8; 1];

    fn deserialize_read_response(&self, data: &[u8]) -> Result<Self::ReadData, FromGattError> {
        <Self::ReadData as FromGatt>::from_gatt(data)
    }

    fn serialize_write_data(&self, data: Self::WriteData) -> impl AsGatt {
        let value = u8::from_le_bytes(data);
        value
    }

    fn id(&self) -> Self::Id {
        STORAGE_DATA_DESCRIPTOR_UUID
    }
}
