use std::thread::spawn;
use dotenv::dotenv;
use tokio::runtime::Runtime;
use homesoil::db::connect;
use homesoil::servers::{run_coap_server, run_socket_server};

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

    spawn(move || {
        println!("Starting CoAP server");

        Runtime::new()
            .expect("Failed to create Tokio runtime")
            .block_on(run_coap_server(&io));
    });

    std::thread::park();

    Ok(())
}