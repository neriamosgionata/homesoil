use anyhow::Result;
use dotenv::dotenv;
use homesoil::auth::{cleanup_expired_tokens, generate_device_secret, rotate_pin};
use homesoil::db::init_pool;
use homesoil::servers::{
    check_for_old_sensor_reads_records, run_coap_server, run_sensor_health_check, run_socket_server,
};
use local_ip_address::local_ip;
use log::info;

#[tokio::main(worker_threads = 4)]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    init_pool();

    let mut current_ip_address = match local_ip() {
        Ok(ip) => ip.to_string(),
        Err(_) => {
            panic!("Error getting local IP address");
        }
    };

    if std::env::var("IS_DEV").is_ok() {
        current_ip_address = "127.0.0.1".to_string();
    }

    let pin = rotate_pin();
    let device_secret = generate_device_secret();
    info!("========================================");
    info!("  Pairing PIN:    {}", pin);
    info!("  Device Secret:  {}", device_secret);
    info!("  PIN rotates when expired (10 minutes)");
    info!("========================================");

    let current_ip_address_socket_io = format!("{}:{}", current_ip_address, 4000);
    let coap_bind_address = format!("0.0.0.0:{}", 5683);

    let io = run_socket_server(String::leak(current_ip_address_socket_io)).await?;

    run_sensor_health_check(&io);

    run_coap_server(
        String::leak(coap_bind_address),
        String::leak(current_ip_address.clone()),
        5683,
        String::leak(device_secret),
        &io,
    )
    .await;

    check_for_old_sensor_reads_records();

    // Periodically clean up expired session tokens
    tokio::spawn(async {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            tokio::task::spawn_blocking(cleanup_expired_tokens).await.ok();
        }
    });

    tokio::signal::ctrl_c().await?;
    info!("Shutting down");

    Ok(())
}
