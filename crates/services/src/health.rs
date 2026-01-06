use core::fmt::Display;

use super::uuid_to_ble_bytes;
use log::{error, info};
use trouble_host::{
    PacketPool,
    gatt::GattConnection,
    prelude::{AsGatt, FromGatt, gatt_service},
    types::gatt_traits::FromGattError,
};
use uuid::Uuid;

const HEALTH_SERVICE_UUID: Uuid = Uuid::from_u128(0xc7d9a5b06c1a4b2c9b3a3d45e6a10000);
pub const HEALTH_STATUS_CHAR_UUID: Uuid = Uuid::from_u128(0xc7d9a5b06c1a4b2c9b3a3d45e6a10001);
pub const HEALTH_PING_CHAR_UUID: Uuid = Uuid::from_u128(0xc7d9a5b06c1a4b2c9b3a3d45e6a10002);

#[gatt_service(uuid = uuid_to_ble_bytes(&HEALTH_SERVICE_UUID))]
pub struct HealthService {
    #[characteristic(uuid = uuid_to_ble_bytes(&HEALTH_STATUS_CHAR_UUID), read, value=true)]
    pub status: bool,
    #[characteristic(uuid = uuid_to_ble_bytes(&HEALTH_PING_CHAR_UUID), notify)]
    pub ping: Pong,
}

impl HealthService {
    pub fn status_handle(&self) -> u16 {
        self.status.handle
    }

    pub fn ping_handle(&self) -> u16 {
        self.ping.handle
    }

    pub fn ping_ccd_handle(&self) -> Option<u16> {
        self.ping.cccd_handle
    }

    pub fn service_uuid_16() -> [u8; 2] {
        let raw = HEALTH_SERVICE_UUID.to_bytes_le();
        [raw[0], raw[1]]
    }

    pub async fn process_status<P: PacketPool>(&self, _conn: &GattConnection<'_, '_, P>) {}

    pub async fn process_ping<P: PacketPool>(&self, conn: &GattConnection<'_, '_, P>) {
        info!("[health_service] starting ping notification...");
        loop {
            if self.ping.notify(conn, &Pong).await.is_err() {
                error!("[health_service] error sending ping notification");
                break;
            };
            embassy_time::Timer::after_secs(1).await;
        }
    }
}

pub struct Pong;

impl AsGatt for Pong {
    const MIN_SIZE: usize = 0;
    const MAX_SIZE: usize = 0;

    fn as_gatt(&self) -> &[u8] {
        &[]
    }
}

impl FromGatt for Pong {
    fn from_gatt(_data: &[u8]) -> Result<Self, FromGattError> {
        Ok(Self)
    }
}

impl Default for Pong {
    fn default() -> Self {
        Self
    }
}

impl Display for Pong {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Pong")
    }
}
// read RSSI (Received Signal Strength Indicator) of the connection.
// if let Ok(rssi) = conn.raw().rssi(stack).await {
//     info!("[custom_task] RSSI: {:?}", rssi);
// } else {
//     info!("[custom_task] error getting RSSI");
//     break;
// };
