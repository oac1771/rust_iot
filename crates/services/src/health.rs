use core::fmt::Display;

use super::uuid_to_ble_bytes;
use log::{error, info};
use trouble_host::{
    Controller, PacketPool, Stack,
    gatt::GattConnection,
    prelude::{AsGatt, FromGatt, descriptors, gatt_service},
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
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, name = "hello", read, value = "Ping Pong", type = &'static str)]
    #[characteristic(uuid = uuid_to_ble_bytes(&HEALTH_PING_CHAR_UUID), notify)]
    pub ping: Pong,
}

impl HealthService {
    pub fn status_handle(&self) -> u16 {
        self.status.handle
    }

    pub fn ping_ccd_handle(&self) -> Option<u16> {
        self.ping.cccd_handle
    }

    pub fn service_uuid_16() -> [u8; 2] {
        let raw = HEALTH_SERVICE_UUID.to_bytes_le();
        [raw[0], raw[1]]
    }

    pub async fn process_ping<P: PacketPool, C: Controller>(
        &self,
        conn: &GattConnection<'_, '_, P>,
        stack: &Stack<'_, C, P>,
    ) {
        info!("[health_service] starting ping notification...");
        loop {
            let rssi = if let Ok(rssi) = conn.raw().rssi(stack).await {
                rssi
            } else {
                error!("[health_service] error getting RSSI");
                break;
            };

            let pong = Pong::new(rssi);

            if self.ping.notify(conn, &pong).await.is_err() {
                error!("[health_service] error sending ping notification");
                break;
            };
            embassy_time::Timer::after_secs(1).await;
        }
    }
}

#[derive(Default)]
pub struct Pong {
    rssi: i8,
}

impl Pong {
    fn new(rssi: i8) -> Self {
        Self { rssi }
    }
}

impl AsGatt for Pong {
    const MIN_SIZE: usize = core::mem::size_of::<i8>();
    const MAX_SIZE: usize = core::mem::size_of::<i8>();

    fn as_gatt(&self) -> &[u8] {
        self.rssi.as_gatt()
    }
}

impl FromGatt for Pong {
    fn from_gatt(data: &[u8]) -> Result<Self, FromGattError> {
        let rssi = i8::from_gatt(data)?;
        Ok(Self { rssi })
    }
}

impl Display for Pong {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Pong (rssi: {})", self.rssi)
    }
}
