#![no_std]
#![no_main]

mod iot;

use embassy_executor::Spawner;
use esp_hal::clock::CpuClock;
use esp_hal::timer::timg::TimerGroup;
use esp_radio::ble::controller::BleConnector;
use iot::run;
use server::config::Config;
use trouble_host::prelude::ExternalController;
use {esp_alloc as _, esp_backtrace as _};

esp_bootloader_esp_idf::esp_app_desc!();

const UUID: Option<&str> = option_env!("UUID");

#[esp_rtos::main]
async fn main(_s: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    esp_alloc::heap_allocator!(size: 72 * 1024);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    #[cfg(target_arch = "riscv32")]
    let software_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);

    esp_rtos::start(
        timg0.timer0,
        #[cfg(target_arch = "riscv32")]
        software_interrupt.software_interrupt0,
    );

    let connector =
        BleConnector::new(peripherals.BT, Default::default()).expect("BLE controller init failed");
    let controller: ExternalController<_, 20> = ExternalController::new(connector);

    let uuid = UUID.unwrap_or("UUID");
    log::info!("UUID: {}", uuid);

    let config = Config::new(uuid.as_bytes());
    run(controller, config).await;
}
