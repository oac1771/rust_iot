#![no_std]
use embassy_futures::select::select;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Receiver, Sender},
};
use embassy_time::Duration;
use log::{error, info, warn};
use services::{health::HealthService, led::LedService};
use trouble_host::prelude::*;
use util::WriteData;

pub const SERVER_NAME: &'static str = "TrouBLE";
pub const ADVERTISE_NAME: &'static str = "Trouble Example";

#[gatt_server]
pub struct Server {
    health_service: HealthService,
    led_service: LedService,
}

impl Server<'_> {
    pub fn init() -> Result<Self, &'static str> {
        let server = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
            name: SERVER_NAME,
            appearance: &appearance::UNKNOWN,
        }))?;

        info!("ping service handle: {}", server.health_service.ping.handle);
        info!(
            "ping ccd service handle: {}",
            server.health_service.ping.cccd_handle.unwrap()
        );
        info!(
            "status service handle: {}",
            server.health_service.status.handle
        );
        info!("led service handle: {}", server.led_service.val.handle);

        Ok(server)
    }

    pub async fn start<'values, C: Controller>(
        self,
        peripheral: &mut Peripheral<'values, C, DefaultPacketPool>,
    ) {
        loop {
            match self.advertise(peripheral).await {
                Ok(conn) => {
                    let payload_channel: Channel<CriticalSectionRawMutex, Payload, 8> =
                        Channel::new();
                    let payload_sender = payload_channel.sender();
                    let payload_receiver = payload_channel.receiver();

                    let gatt_driver = drive_connection(&conn, payload_sender);
                    let payload_driver = self.handle_payload(&conn, payload_receiver);

                    select(gatt_driver, payload_driver).await;
                }
                Err(e) => {
                    error!("[adv] error: {:?}", e);
                }
            }
        }
    }

    pub async fn handle_payload<'stack, 'server, P: PacketPool>(
        &self,
        conn: &GattConnection<'_, '_, P>,
        payload_receiver: Receiver<'_, CriticalSectionRawMutex, Payload, 8>,
    ) {
        loop {
            match payload_receiver.receive().await {
                Payload::Read { handle } => {
                    if handle == self.health_service.status_handle() {
                        self.health_service.process_status(conn).await;
                    } else if handle == self.health_service.ping_handle() {
                        self.health_service.process_ping(conn).await;
                    } else {
                        warn!("Read payload handle did not match known handle")
                    }
                }
                Payload::Write { handle, write_data } => {
                    if handle == self.led_service.val_handle() {
                        self.led_service.process(write_data).await;
                    } else if Some(handle) == self.health_service.ping_ccd_handle() {
                        self.health_service.process_ping(conn).await;
                    } else {
                        warn!("Write payload handle did not match known handle")
                    }
                }
                Payload::Other => continue,
            }
        }
    }

    pub async fn advertise<'values, C: Controller>(
        &self,
        peripheral: &mut Peripheral<'values, C, DefaultPacketPool>,
    ) -> Result<GattConnection<'values, '_, DefaultPacketPool>, BleHostError<C::Error>> {
        let mut advertiser_data = [0; 31];
        let mut scan_data = [0; 31];

        let scan_len = AdStructure::encode_slice(
            &[AdStructure::CompleteLocalName(ADVERTISE_NAME.as_bytes())],
            &mut scan_data,
        )?;

        let adv_len = AdStructure::encode_slice(
            &[
                AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
                AdStructure::ServiceUuids16(&[HealthService::service_uuid_16()]),
                AdStructure::ServiceUuids16(&[LedService::service_uuid_16()]),
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
        let conn = advertiser.accept().await?.with_attribute_server(&self)?;
        info!("[adv] connection established");
        Ok(conn)
    }
}

async fn drive_connection<P: PacketPool>(
    conn: &GattConnection<'_, '_, P>,
    sender: Sender<'_, CriticalSectionRawMutex, Payload, 8>,
) {
    loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => {
                info!("[gatt] disconnected: {:?}", reason);
                break;
            }
            GattConnectionEvent::Gatt { event } => {
                let payload = match Payload::try_from(&event) {
                    Ok(payload) => payload,
                    Err(err) => {
                        error!("Unable to parse payload: {}", err);
                        continue;
                    }
                };
                sender.send(payload).await;

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

pub enum Payload {
    Read { handle: u16 },
    Write { handle: u16, write_data: WriteData },
    Other,
}

impl<P: PacketPool> TryFrom<&GattEvent<'_, '_, P>> for Payload {
    type Error = heapless::CapacityError;

    fn try_from(value: &GattEvent<'_, '_, P>) -> Result<Self, Self::Error> {
        match value {
            GattEvent::Read(event) => Ok(Self::Read {
                handle: event.handle(),
            }),
            GattEvent::Write(event) => {
                let handle = event.handle();
                let mut write_data = WriteData::new();
                write_data.extend_from_slice(event.data())?;
                Ok(Self::Write { handle, write_data })
            }
            GattEvent::Other(_) => Ok(Self::Other),
        }
    }
}
