use dotenv::dotenv;
use socketioxide::extract::SocketRef;
use tokio::runtime::Runtime;
use homesoil::db::connect;
use homesoil::servers::{run_coap_server, run_socket_server};

#[tokio::main]
async fn main() {
    dotenv().ok();
    match connect() {
        Ok(_) => {}
        Err(e) => {
            panic!("Error connecting to database: {}", e);
        }
    }

    let io = run_socket_server().await;

    io.ns("/", |socket: SocketRef| {
        Runtime::new()
            .expect("Failed to create Tokio runtime")
            .block_on(
                run_coap_server(&socket)
            );
    });

    loop {}
}