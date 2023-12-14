use std::pin::Pin;
use rust_socketio::{Payload, asynchronous::Client};
use serde_json::json;
use futures_util::FutureExt;

pub fn main_socket_message_callback(payload: Payload, socket: Client) -> Pin<Box<(dyn futures_util::Future<Output=()> + Send + 'static)>> {
    async move {
        match payload {
            Payload::String(str) => println!("Received: {}", str),
            Payload::Binary(bin_data) => println!("Received bytes: {:#?}", bin_data),
        }
        socket
            .emit("test", json!({"ack": true}))
            .await.expect("Server unreachable");
    }
        .boxed()
}
