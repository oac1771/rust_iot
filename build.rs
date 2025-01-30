fn main() {
    let ssid = std::env::var("SSID").unwrap_or_else(|_| "ssid".to_string());
    let password = std::env::var("PASSWORD").unwrap_or_else(|_| "password".to_string());

    println!("cargo:rustc-link-arg=-Tlinkall.x");
    println!("cargo:rustc-env=SSID={}", ssid);
    println!("cargo:rustc-env=PASSWORD={}", password);
}
