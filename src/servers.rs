use std::env;
use std::net::SocketAddr;
use std::thread::spawn;
use crate::handlers::path_handler;
use crate::Server;
use socketioxide::SocketIo;
use anyhow::Result;
use axum::routing::get;
use axum::Router;
use axum::Server as AxumServer;
use socketioxide::extract::SocketRef;
use tokio::runtime::Runtime;

pub async fn run_socket_server() -> Result<SocketIo> {
    let address = env::var("SOCKET_IO_SERVER_ADDRESS").expect("SOCKET_IO_SERVER_ADDRESS must be set");

    let (layer, io) = SocketIo::new_layer();

    io.ns("/", |_socket: SocketRef| {
        println!("Socket connected : {:?}", _socket.id);
    });

    spawn(move || {
        Runtime::new()
            .expect("Failed to create Tokio runtime")
            .block_on(async move {
                let app = Router::new()
                    .route("/", get(|| async { "OK" }))
                    .layer(layer);

                AxumServer::bind(&address.clone().parse::<SocketAddr>().expect("Failed to parse SocketIO server address"))
                    .serve(app.into_make_service())
                    .await
                    .unwrap();
            });
    });

    Ok(io)
}


pub async fn run_coap_server(socket: &SocketIo) {
    let address = env::var("COAP_SERVER_ADDRESS").expect("COAP_SERVER_ADDRESS must be set");

    let mut server = Server::new(address.clone()).unwrap();

    server.run(
        |request| async move {
            let request_ref = &request;

            let payload = path_handler(socket, &request_ref).await;

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
}