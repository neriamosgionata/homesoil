use coap::Server;
use tokio::runtime::Runtime;
use homesoil::handlers::{get_handlers, path_handler};
use dotenv::dotenv;

fn main() {
    println!("Hello, world!");

    dotenv().ok();

    let address = "127.0.0.1:5683";

    println!("Connected to database");

    Runtime::new().unwrap().block_on(async move {
        let mut server = Server::new(address).unwrap();

        println!("Server up on {}", address);

        server.run(
            |request| async move {
                let res = path_handler(&request, get_handlers());
                match request.response {
                    Some(mut message) => {
                        match res {
                            Some(payload) => {
                                message.message.payload = payload.as_bytes().to_vec();
                            }
                            None => {
                                message.message.payload = b"Error".to_vec();
                            }
                        }
                        Some(message)
                    }
                    _ => None
                }
            },
        )
            .await
            .expect("Failed to create server");
    });
}
