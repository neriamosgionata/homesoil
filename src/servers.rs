use crate::actuator_handlers::ping_actuator;
use crate::actuator_methods::get_all_registered_actuators;
use crate::events::{
    register_all_callbacks, ALL_ACTUATORS_EVENT, ALL_LAST_SENSOR_READINGS_EVENT, ALL_SENSORS_EVENT,
};
use crate::handlers::path_handler;
use crate::sensor_handlers::ping_sensor;
use crate::sensor_methods::{get_all_last_sensor_readings, get_all_registered_sensors};
use crate::Server;
use anyhow::Result;
use axum::routing::get;
use axum::Router;
use axum::Server as AxumServer;
use axum_util::cors::CorsLayer;
use serde::Deserialize;
use serde_json::json;
use socketioxide::extract::{Data, SocketRef};
use socketioxide::{SocketIo, TransportType};
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread::{spawn, JoinHandle};
use std::time::Duration;
use tokio::runtime::Runtime;

#[derive(Debug, Deserialize)]
struct AuthData {
    token: String,
}

pub async fn run_socket_server(address: &'static str) -> Result<SocketIo> {
    let (layer, io) = SocketIo::builder()
        .connect_timeout(Duration::from_secs(30))
        .req_path("/socket.io")
        .transports([TransportType::Websocket, TransportType::Polling])
        .build_layer();

    io.ns("/", move |socket: SocketRef, Data(auth): Data<AuthData>| {
        let login_token = std::env::var("LOGIN_TOKEN").expect("LOGIN_TOKEN must be set");

        if auth.token.is_empty() || auth.token != login_token {
            println!("Invalid token, disconnecting socket : {:?}", socket.id);
            socket.disconnect().ok();
            return;
        }

        println!("Socket connected : {:?}", socket.id);

        register_all_callbacks(&socket);

        socket.on_disconnect(|socket: SocketRef| {
            println!("Socket disconnected : {:?}", socket.id);
        });

        if let Ok(sensors) = get_all_registered_sensors() {
            let _: Result<_, _> = socket.emit(
                ALL_SENSORS_EVENT,
                json!({
                    "sensors": sensors,
                }),
            );
        }

        if let Ok(sensor_reads) = get_all_last_sensor_readings() {
            let _: Result<_, _> = socket.emit(
                ALL_LAST_SENSOR_READINGS_EVENT,
                json!({
                    "sensor_reads": sensor_reads,
                }),
            );
        }

        if let Ok(actuators) = get_all_registered_actuators() {
            let _: Result<_, _> = socket.emit(
                ALL_ACTUATORS_EVENT,
                json!({
                    "actuators": actuators,
                }),
            );
        }
    });

    spawn(move || {
        Runtime::new()
            .expect("Failed to create Tokio runtime")
            .block_on(async {
                println!("Starting SocketIO server on {}", address);

                let app = Router::new()
                    .route("/", get(|| async { "OK" }))
                    .layer(layer)
                    .layer(CorsLayer);

                AxumServer::bind(
                    &address
                        .parse::<SocketAddr>()
                        .expect("Failed to parse SocketIO server address"),
                )
                .serve(app.into_make_service())
                .await
                .unwrap();

                println!("SocketIO server stopped");
            });
    });

    Ok(io)
}

pub async fn run_sensor_health_check(socket: &SocketIo) -> JoinHandle<()> {
    let boxed_socket = Box::new(socket.clone());

    spawn(move || loop {
        if let Ok(sensors) = get_all_registered_sensors() {
            sensors.iter().for_each(|sensor| {
                ping_sensor(sensor, boxed_socket.as_ref());
            });
        }

        if let Ok(actuators) = get_all_registered_actuators() {
            actuators.iter().for_each(|actuator| {
                ping_actuator(actuator, boxed_socket.as_ref());
            });
        }

        std::thread::sleep(Duration::from_secs(5));
    })
}

pub async fn run_coap_server(address: &'static str, socket: &SocketIo) {
    let boxed_socket = Arc::new(socket.clone());

    spawn(move || {
        println!("Starting CoAP server on {}", address);

        Runtime::new()
            .expect("Failed to create Tokio runtime")
            .block_on(async {
                let mut server = Server::new(address).unwrap();

                server
                    .run(|request| async {
                        let request_ref = &request;

                        let payload = path_handler(boxed_socket.as_ref(), request_ref).await;

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
                            _ => None,
                        }
                    })
                    .await
                    .expect("Failed to create server");
            });
    });
}

pub async fn check_for_old_sensor_reads_records() {
    spawn(move || loop {
        let _: Result<_, _> = crate::sensor_methods::delete_old_sensor_reads_records();

        std::thread::sleep(Duration::from_secs(3600));
    });
}
