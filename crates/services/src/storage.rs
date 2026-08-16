use super::uuid_to_ble_bytes;
use log::info;
use trouble_host::{
    prelude::{descriptors, gatt_service},
    types::gatt_traits::{AsGatt, FromGatt, FromGattError},
};
use util::WriteData;
use uuid::Uuid;

const STORAGE_SERVICE_UUID: Uuid = Uuid::from_u128(0xc7d9a5b06c1a4b2c9b3a3d45e6a20000);
pub const STORAGE_STATUS_CHAR_UUID: Uuid = Uuid::from_u128(0xc7d9a5b06c1a4b2c9b3a3d45e6a20001);

#[gatt_service(uuid = uuid_to_ble_bytes(&STORAGE_SERVICE_UUID))]
pub struct StorageService {
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, read, write, value = StorageServiceDataDescriptor, type = StorageServiceDataDescriptor)]
    #[characteristic(uuid = uuid_to_ble_bytes(&STORAGE_STATUS_CHAR_UUID), read, write)]
    pub data: u8,
}

impl StorageService {
    pub fn data_handle(&self) -> u16 {
        self.data.handle
    }

    pub async fn process(&self, _write_data: WriteData) {
        info!("[led_service] notifying connection of status");
    }
}

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
