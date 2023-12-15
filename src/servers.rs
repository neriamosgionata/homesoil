use std::env;
use socketioxide::extract::SocketRef;
use crate::handlers::path_handler;
use crate::Server;
use socketioxide::SocketIo;


pub async fn run_socket_server() -> SocketIo {
    let address = env::var("SOCKET_IO_SERVER_ADDRESS").expect("SOCKET_IO_SERVER_ADDRESS must be set");

    println!("Starting SocketIO server on {}", address);

    let (_layer, io) = SocketIo::builder()
        .build_layer();

    io
}


pub async fn run_coap_server(socket: &SocketRef) {
    let address = env::var("COAP_SERVER_ADDRESS").expect("COAP_SERVER_ADDRESS must be set");

    println!("Running COAP server on {}", address);

    let mut server = Server::new(address.clone()).unwrap();

    server.run(
        |request| async move {
            let request_ref = &request;

            let payload = path_handler(&socket, &request_ref).await;

            match request.response {
                Some(mut message) => {
                    match payload {
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

    println!("COAP server stopped");
}