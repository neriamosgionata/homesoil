use anyhow::Result;
use dotenv::dotenv;
use homesoil::auth::{generate_device_secret, generate_pin};
use homesoil::db::connect;
use homesoil::servers::{
    check_for_old_sensor_reads_records, run_coap_server, run_sensor_health_check, run_socket_server,
};
use local_ip_address::local_ip;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    match connect() {
        Ok(_) => {}
        Err(e) => {
            panic!("Error connecting to database: {}", e);
        }
    }

    let mut current_ip_address = match local_ip() {
        Ok(ip) => ip.to_string(),
        Err(_) => {
            panic!("Error getting local IP address");
        }
    };

    if std::env::var("IS_DEV").is_ok() {
        current_ip_address = "127.0.0.1".to_string();
    }

    let pin = generate_pin();
    let device_secret = generate_device_secret();
    println!("========================================");
    println!("  Pairing PIN:    {}", pin);
    println!("  Device Secret:  {}", device_secret);
    println!("========================================");

    let current_ip_address_socket_io = format!("{}:{}", current_ip_address, 4000);
    let coap_bind_address = format!("0.0.0.0:{}", 5683);

    let io = run_socket_server(String::leak(current_ip_address_socket_io), String::leak(pin)).await?;

    run_sensor_health_check(&io).await;

    run_coap_server(
        String::leak(coap_bind_address),
        String::leak(current_ip_address.clone()),
        5683,
        String::leak(device_secret),
        &io,
    )
    .await;

    check_for_old_sensor_reads_records().await;

    std::thread::park();

    Ok(())
}
