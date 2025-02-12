use anyhow::Result;
use dotenv::dotenv;
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

    let socket_port = std::env::var("SOCKET_PORT").unwrap_or("4000".to_string());
    let coap_port = std::env::var("COAP_PORT").unwrap_or("8683".to_string());

    let current_ip_address_socket_io = format!("{}:{}", current_ip_address, socket_port);
    let current_ip_address_coap = format!("{}:{}", current_ip_address, coap_port);

    let io = run_socket_server(String::leak(current_ip_address_socket_io)).await?;

    run_sensor_health_check(&io).await;

    run_coap_server(String::leak(current_ip_address_coap), &io).await;

    check_for_old_sensor_reads_records().await;

    std::thread::park();

    Ok(())
}
