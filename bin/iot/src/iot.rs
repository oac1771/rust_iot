use embassy_futures::{join::join, select::select};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Receiver, Sender},
};
use embassy_time::Duration;
use log::{error, info, warn};
use services::{health::HealthService, led::LedService};
use trouble_host::prelude::*;
use util::WriteData;

/// Max number of connections
const CONNECTIONS_MAX: usize = 1;

/// Max number of L2CAP channels.
const L2CAP_CHANNELS_MAX: usize = 2; // Signal + att

#[gatt_server]
struct Server {
    health_service: HealthService,
    led_service: LedService,
}

impl Server<'_> {
    async fn handle_payload<'stack, 'server, P: PacketPool>(
        &self,
        conn: &GattConnection<'_, '_, P>,
        payload_receiver: Receiver<'_, CriticalSectionRawMutex, Payload, 8>,
    ) {
        loop {
            match payload_receiver.receive().await {
                Payload::Read { handle } => {
                    if handle == self.health_service.status_handle() {
                        self.health_service.process(conn).await;
                    } else {
                        warn!("Read payload handle did not match known handle")
                    }
                },
                Payload::Write { handle, write_data } => {
                    if handle == self.led_service.val_handle() {
                        self.led_service.process(write_data).await;
                    } else {
                        warn!("Write payload handle did not match known handle")
                    }
                },
                Payload::Other => continue,
            }
        }
    }
}

/// Run the BLE stack.
pub async fn run<C>(controller: C)
where
    C: Controller,
{
    // Using a fixed "random" address can be useful for testing. In real scenarios, one would
    // use e.g. the MAC 6 byte array as the address (how to get that varies by the platform).
    let address: Address = Address::random([0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xff]);
    info!("Our address = {:?}", address);

    let mut resources: HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> =
        HostResources::new();
    let stack = trouble_host::new(controller, &mut resources).set_random_address(address);
    let Host {
        mut peripheral,
        runner,
        ..
    } = stack.build();

    info!("Starting advertising and GATT service");
    let server = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name: "TrouBLE",
        appearance: &appearance::UNKNOWN,
    }))
    .unwrap();

    let _ = join(ble_task(runner), async {
        loop {
            match advertise("Trouble Example", &mut peripheral, &server).await {
                Ok(conn) => {
                    let payload_channel: Channel<CriticalSectionRawMutex, Payload, 8> = Channel::new();
                    let payload_sender = payload_channel.sender();
                    let payload_receiver = payload_channel.receiver();

                    let gatt_driver = drive_connection(&conn, payload_sender);
                    let payload_driver = server.handle_payload(&conn, payload_receiver);

                    select(gatt_driver, payload_driver).await;
                }
                Err(e) => {
                    error!("[adv] error: {:?}", e);
                }
            }
        }
    })
    .await;
}

/// Create an advertiser to use to connect to a BLE Central, and wait for it to connect.
async fn advertise<'values, 'server, C: Controller>(
    name: &'values str,
    peripheral: &mut Peripheral<'values, C, DefaultPacketPool>,
    server: &'server Server<'values>,
) -> Result<GattConnection<'values, 'server, DefaultPacketPool>, BleHostError<C::Error>> {
    let mut advertiser_data = [0; 31];
    let mut scan_data = [0; 31];

    let scan_len = AdStructure::encode_slice(
        &[AdStructure::CompleteLocalName(name.as_bytes())],
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
    let conn = advertiser.accept().await?.with_attribute_server(server)?;
    info!("[adv] connection established");
    Ok(conn)
}

/// This is a background task that is required to run forever alongside any other BLE tasks.
///
/// ## Alternative
///
/// If you didn't require this to be generic for your application, you could statically spawn this with i.e.
///
/// ```rust,ignore
///
/// #[embassy_executor::task]
/// async fn ble_task(mut runner: Runner<'static, SoftdeviceController<'static>>) {
///     runner.run().await;
/// }
///
/// spawner.must_spawn(ble_task(runner));
/// ```
async fn ble_task<C: Controller, P: PacketPool>(mut runner: Runner<'_, C, P>) {
    loop {
        if let Err(e) = runner.run().await {
            panic!("[ble_task] error: {:?}", e);
        }
    }
}

/// Stream Events until the connection closes.
///
/// This function will handle the GATT events and process them.
/// This is how we interact with read and write requests.
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
                        continue
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

enum Payload {
    Read { handle: u16 },
    Write { handle: u16, write_data: WriteData },
    Other
}

impl<P: PacketPool> TryFrom<&GattEvent<'_, '_, P>> for Payload {
    type Error = heapless::CapacityError;

    fn try_from(value: &GattEvent<'_, '_, P>) -> Result<Self, Self::Error> {
        match value {
            GattEvent::Read(event) => Ok(Self::Read { handle: event.handle() }),
            GattEvent::Write(event) => {
                let handle = event.handle();
                let mut write_data = WriteData::new();
                write_data.extend_from_slice(event.data())?;
                Ok(Self::Write { handle, write_data })
            },
            GattEvent::Other(_) => Ok(Self::Other)
        }
    }
}