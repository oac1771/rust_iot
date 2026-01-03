use super::uuid_to_ble_bytes;
use log::info;
use trouble_host::prelude::gatt_service;
use util::WriteData;
use uuid::Uuid;

const LED_SERVICE_UUID: Uuid = Uuid::from_u128(0xc7d9a5b06c1a4b2c9b3a3d45e6a20000);
pub const LED_STATUS_CHAR_UUID: Uuid = Uuid::from_u128(0xc7d9a5b06c1a4b2c9b3a3d45e6a20001);

#[gatt_service(uuid = uuid_to_ble_bytes(&LED_SERVICE_UUID))]
pub struct LedService {
    #[characteristic(uuid = uuid_to_ble_bytes(&LED_STATUS_CHAR_UUID), write)]
    pub val: bool,
}

impl LedService {
    pub fn val_handle(&self) -> u16 {
        self.val.handle
    }

    pub fn service_uuid_16() -> [u8; 2] {
        let raw = LED_SERVICE_UUID.to_bytes_le();
        [raw[0], raw[1]]
    }

    pub async fn process(&self, _write_data: WriteData) {
        info!("[led_service] notifying connection of status");
    }
}
