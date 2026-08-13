#![no_std]

pub mod config;

use embassy_futures::select::select;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Receiver, Sender},
};
use embassy_time::Duration;
use log::{error, info, warn};
use services::{health::HealthService, storage::StorageService};
use trouble_host::prelude::*;
use util::WriteData;

use crate::config::Config;

pub const SERVER_NAME: &str = "IOT_DEVICE";

#[gatt_server]
pub struct Server {
    health_service: HealthService,
    storage_service: StorageService,
}

impl Server<'_> {
    pub fn init() -> Result<Self, &'static str> {
        let server = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
            name: SERVER_NAME,
            appearance: &appearance::UNKNOWN,
        }))?;

        Ok(server)
    }

    pub async fn start<'values, C: Controller>(
        self,
        peripheral: &mut Peripheral<'values, C, DefaultPacketPool>,
        stack: &Stack<'_, C, DefaultPacketPool>,
        config: Config<'_>,
    ) {
        loop {
            match self.advertise(peripheral, &config).await {
                Ok(conn) => {
                    let write_payload_channel: Channel<CriticalSectionRawMutex, WritePayload, 8> =
                        Channel::new();

                    let write_payload_sender = write_payload_channel.sender();
                    let write_payload_receiver = write_payload_channel.receiver();

                    let gatt_driver = drive_connection(&conn, &write_payload_sender);
                    let payload_driver = self.handle_payload(&conn, &write_payload_receiver, stack);

                    select(gatt_driver, payload_driver).await;
                }
                Err(e) => {
                    error!("[adv] error: {:?}", e);
                }
            }
        }
    }

    pub async fn handle_payload<P: PacketPool, C: Controller>(
        &self,
        conn: &GattConnection<'_, '_, P>,
        write_payload_receiver: &Receiver<'_, CriticalSectionRawMutex, WritePayload, 8>,
        stack: &Stack<'_, C, P>,
    ) {
        loop {
            let WritePayload { handle, write_data } = write_payload_receiver.receive().await;
            if handle == self.storage_service.val_handle() {
                self.storage_service.process(write_data).await;
            } else if Some(handle) == self.health_service.ping_ccd_handle() {
                self.health_service.process_ping(conn, stack).await;
            } else {
                warn!("Write payload handle did not match known handle")
            }
        }
    }

    pub async fn advertise<'values, C: Controller>(
        &self,
        peripheral: &mut Peripheral<'values, C, DefaultPacketPool>,
        config: &Config<'_>,
    ) -> Result<GattConnection<'values, '_, DefaultPacketPool>, BleHostError<C::Error>> {
        let mut advertiser_data = [0; 31];
        let mut scan_data = [0; 31];

        let scan_len = AdStructure::encode_slice(
            &[AdStructure::CompleteLocalName(config.uuid())],
            &mut scan_data,
        )?;

        let adv_len = AdStructure::encode_slice(
            &[
                AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
                AdStructure::CompleteLocalName(config.uuid()),
            ],
            &mut advertiser_data[..],
        )?;

        let advertiser = peripheral
            .advertise(
                &AdvertisementParameters {
                    interval_min: Duration::from_millis(20),
                    interval_max: Duration::from_millis(20),
                    ..Default::default()
                },
                Advertisement::ConnectableScannableUndirected {
                    adv_data: &advertiser_data[..adv_len],
                    scan_data: &scan_data[..scan_len],
                },
            )
            .await?;
        info!("[adv] advertising");
        let conn = advertiser.accept().await?.with_attribute_server(self)?;
        info!("[adv] connection established");
        Ok(conn)
    }
}

async fn drive_connection<P: PacketPool>(
    conn: &GattConnection<'_, '_, P>,
    write_payload_sender: &Sender<'_, CriticalSectionRawMutex, WritePayload, 8>,
) {
    loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => {
                info!("[gatt] disconnected: {:?}", reason);
                break;
            }
            GattConnectionEvent::Gatt { event } => {
                match &event {
                    GattEvent::Write(e) => {
                        let mut write_data = WriteData::new();

                        if let Err(err) = write_data.extend_from_slice(e.data()) {
                            error!("Error copying write data: {}", err);
                        };
                        let write_payload = WritePayload {
                            handle: e.handle(),
                            write_data,
                        };
                        write_payload_sender.send(write_payload).await;
                    }
                    GattEvent::Other(_) => {}
                    GattEvent::NotAllowed(_) => {}
                    _ => {}
                }

                match event.accept() {
                    Ok(reply) => {
                        info!("[gatt] reply sent!");
                        reply.send().await
                    }
                    Err(e) => error!("[gatt] error sending response: {:?}", e),
                };
            }
            _ => {}
        }
    }
}

pub struct WritePayload {
    handle: u16,
    write_data: WriteData,
}
