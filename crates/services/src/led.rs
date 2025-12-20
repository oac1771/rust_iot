use trouble_host::prelude::{gatt_service, service};

#[gatt_service(uuid = service::BATTERY)]
pub struct LedService {
    #[characteristic(uuid = "408813df-5dd4-1f87-ec11-cdb001100000", write, read, notify)]
    status: bool,
}
