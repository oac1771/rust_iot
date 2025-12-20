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
    pub fn service_uuid_16() -> [u8; 16] {
        HEALTH_SERVICE_UUID.to_bytes_le().try_into().unwrap()
    }
}

const fn uuid_to_ble_bytes(uuid: &Uuid) -> [u8; 16] {
    let b = *uuid.as_bytes();
    [
        b[15], b[14], b[13], b[12],
        b[11], b[10], b[9],  b[8],
        b[7],  b[6],  b[5],  b[4],
        b[3],  b[2],  b[1],  b[0],
    ]
}