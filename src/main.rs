use dotenv::dotenv;
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

    let socket  = run_socket_server().await;

    run_coap_server(&socket).await;

    loop {}
}