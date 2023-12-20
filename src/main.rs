use dotenv::dotenv;
use homesoil::db::connect;
use homesoil::servers::{run_coap_server, run_sensor_health_check, run_socket_server};
use local_ip_address::local_ip;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    match connect() {
        Ok(_) => {}
        Err(e) => {
            panic!("Error connecting to database: {}", e);
        }
    }

    let current_ip_address = match local_ip() {
        Ok(ip) => ip.to_string(),
        Err(_) => {
            panic!("Error getting local IP address");
        }
    };

    let current_ip_address_socket_io = format!("{}:4000", current_ip_address);
    let current_ip_address_coap = format!("{}:5683", current_ip_address);

    let io = run_socket_server(&current_ip_address_socket_io).await?;

    run_sensor_health_check(&io).await;

    run_coap_server(&current_ip_address_coap, &io).await;

    std::thread::park();

    Ok(())
}