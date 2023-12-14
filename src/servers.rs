use std::env;
use coap::Server;
use futures_util::FutureExt;
use rust_socketio::asynchronous::{Client, ClientBuilder};
use rust_socketio::Event;
use crate::handlers::path_handler;
use crate::socket::main_socket_message_callback;

pub async fn run_socket_server() -> Client {
    let address = env::var("SOCKET_IO_SERVER_ADDRESS").expect("SOCKET_IO_SERVER_ADDRESS must be set");

    println!("Running SocketIO server on {}", address);

    match ClientBuilder::new(address.clone())
        .namespace("/admin")
        .on(Event::Connect, |payload, _socket| {
            async move {
                println!("Connected to server, Payload: {:?}", payload);
            }
                .boxed()
        })
        .on(Event::Close, |payload, _socket| {
            async move {
                println!("Disconnected from server, Payload: {:?}", payload);
            }
                .boxed()
        })
        .on(Event::Message, main_socket_message_callback)
        .connect()
        .await
    {
        Ok(socket) => socket,
        Err(e) => {
            panic!("Error connecting to SocketIO server: {}", e);
        }
    }
}


pub async fn run_coap_server(socket: &Client) {
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