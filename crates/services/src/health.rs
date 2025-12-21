use super::uuid_to_ble_bytes;
use trouble_host::prelude::gatt_service;
use uuid::Uuid;

const HEALTH_SERVICE_UUID: Uuid = Uuid::from_u128(0xc7d9a5b06c1a4b2c9b3a3d45e6a10000);
pub const HEALTH_STATUS_CHAR_UUID: Uuid = Uuid::from_u128(0xc7d9a5b06c1a4b2c9b3a3d45e6a10001);


#[gatt_service(uuid = uuid_to_ble_bytes(&HEALTH_SERVICE_UUID))]
pub struct HealthService {
    #[characteristic(uuid = uuid_to_ble_bytes(&HEALTH_STATUS_CHAR_UUID), notify)]
    pub status: bool,
}

impl HealthService {
    pub fn service_uuid_16() -> [u8; 2] {
        let raw = HEALTH_SERVICE_UUID.to_bytes_le();
        [raw[0], raw[1]]
    }
}
