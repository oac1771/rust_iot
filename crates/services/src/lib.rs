#![no_std]
pub mod health;
pub mod led;

const fn uuid_to_ble_bytes(uuid: &uuid::Uuid) -> [u8; 16] {
    let b = *uuid.as_bytes();
    [
        b[15], b[14], b[13], b[12],
        b[11], b[10], b[9],  b[8],
        b[7],  b[6],  b[5],  b[4],
        b[3],  b[2],  b[1],  b[0],
    ]
}