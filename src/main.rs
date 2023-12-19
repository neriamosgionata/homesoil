use dotenv::dotenv;
use homesoil::db::connect;
use homesoil::servers::{run_coap_server, run_sensor_health_check, run_socket_server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    match connect() {
        Ok(_) => {}
        Err(e) => {
            panic!("Error connecting to database: {}", e);
        }
    }

    println!("Starting SocketIO server");

    let io = run_socket_server().await?;

    println!("SocketIO server started");

    run_sensor_health_check(&io).await;

    run_coap_server(&io).await;

    std::thread::park();

    Ok(())
}