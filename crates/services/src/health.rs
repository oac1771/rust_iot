use super::uuid_to_ble_bytes;
use log::info;
use trouble_host::{PacketPool, prelude::gatt_service, gatt::GattConnection};
use uuid::Uuid;

const HEALTH_SERVICE_UUID: Uuid = Uuid::from_u128(0xc7d9a5b06c1a4b2c9b3a3d45e6a10000);
pub const HEALTH_STATUS_CHAR_UUID: Uuid = Uuid::from_u128(0xc7d9a5b06c1a4b2c9b3a3d45e6a10001);

#[gatt_service(uuid = uuid_to_ble_bytes(&HEALTH_SERVICE_UUID))]
pub struct HealthService {
    #[characteristic(uuid = uuid_to_ble_bytes(&HEALTH_STATUS_CHAR_UUID), read, notify)]
    pub status: bool,
}

impl HealthService {
    pub fn status_handle(&self) -> u16 {
        self.status.handle
    }

    pub fn service_uuid_16() -> [u8; 2] {
        let raw = HEALTH_SERVICE_UUID.to_bytes_le();
        [raw[0], raw[1]]
    }

    pub async fn process<P: PacketPool>(&self, conn: &GattConnection<'_, '_, P>, ) {
        loop {
            info!("[health_service] notifying connection of status");
            if self.status.notify(conn, &true).await.is_err() {
                info!("[health_service] error notifying connection");
                break;
            };
            embassy_time::Timer::after_secs(1).await;
        }
    }
}

            // read RSSI (Received Signal Strength Indicator) of the connection.
            // if let Ok(rssi) = conn.raw().rssi(stack).await {
            //     info!("[custom_task] RSSI: {:?}", rssi);
            // } else {
            //     info!("[custom_task] error getting RSSI");
            //     break;
            // };
