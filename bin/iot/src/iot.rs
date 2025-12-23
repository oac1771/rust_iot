use embassy_futures::join::join;
use embassy_time::Duration;
use log::{error, info, trace, warn};
use services::{health::HealthService, led::LedService};
use trouble_host::prelude::*;

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
    async fn handle_event<'stack, 'server, P: PacketPool>(&self, event: &GattEvent<'stack, 'server, P>, conn: &GattConnection<'_, '_, P>) {
        match event {
            GattEvent::Read(event) => {
                if event.handle() == self.health_service.status.handle {
                    info!("health service read");
                    custom_task(&self, conn).await;
                }
            },
            GattEvent::Write(event) => { 
                if event.handle() == self.led_service.val.handle {
                    info!("led service write")

                }
            },
            _ => {}
        }

    }

    // // this should return array of uuids so you dont have to hard code them in advertise
    // fn services_uuids() -> [Services; 2] {
    //     [Services::Health, Services::Led]
    // }
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
                    handle_connection(&conn, &server).await
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
async fn gatt_events_task<P: PacketPool>(
    server: &Server<'_>,
    conn: &GattConnection<'_, '_, P>,
) -> Result<(), Error> {
    let status = server.health_service.status;
    loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => {
                info!("[gatt] disconnected: {:?}", reason);
                break;
            }
            GattConnectionEvent::Gatt { event } => {
                match &event {
                    GattEvent::Read(event) => {
                        if event.handle() == status.handle {
                            let value = server.get(&status);
                            info!("[gatt] Read Event to Level Characteristic: {:?}", value);
                        }
                    }
                    GattEvent::Write(event) => {
                        if event.handle() == status.handle {
                            info!(
                                "[gatt] Write Event to Level Characteristic: {:?}",
                                event.data()
                            );
                        }
                    }
                    GattEvent::Other(_) => {
                        info!("[gatt] Other event");
                    }
                };
                // This step is also performed at drop(), but writing it explicitly is necessary
                // in order to ensure reply is sent.
                match event.accept() {
                    Ok(reply) => {
                        info!("[gatt] reply sent!");
                        reply.send().await
                    }
                    Err(e) => warn!("[gatt] error sending response: {:?}", e),
                };
            }
            _ => {} // ignore other Gatt Connection Events
        }
    }
    Ok(())
}

/// Example task to use the BLE notifier interface.
/// This task will notify the connected central of a counter value every 2 seconds.
/// It will also read the RSSI value every 2 seconds.
/// and will stop when the connection is closed by the central or an error occurs.
async fn custom_task<P: PacketPool>(
    server: &Server<'_>,
    conn: &GattConnection<'_, '_, P>,
    // stack: &Stack<'_, C, P>,
) {
    let status = server.health_service.status;
    loop {
        info!("[custom_task] notifying connection of status");
        if status.notify(conn, &true).await.is_err() {
            info!("[custom_task] error notifying connection");
            break;
        };
        // read RSSI (Received Signal Strength Indicator) of the connection.
        // if let Ok(rssi) = conn.raw().rssi(stack).await {
        //     info!("[custom_task] RSSI: {:?}", rssi);
        // } else {
        //     info!("[custom_task] error getting RSSI");
        //     break;
        // };
        embassy_time::Timer::after_secs(2).await;
    }
}

async fn handle_connection<P: PacketPool>(conn: &GattConnection<'_, '_, P>, server: &Server<'_>) {
    loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => {
                info!("[gatt] disconnected: {:?}", reason);
                break;
            }
            GattConnectionEvent::Gatt { event } => {
                
                server.handle_event(&event, &conn).await;

                match event.accept() {
                    Ok(reply) => {
                        trace!("[gatt] reply sent!");
                        reply.send().await;
                    }
                    Err(e) => error!("[gatt] error sending response: {:?}", e),
                };
            }
            _ => {}
        }
    }
}

