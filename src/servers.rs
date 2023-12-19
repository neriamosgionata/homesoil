use std::env;
use std::net::SocketAddr;
use std::thread::{JoinHandle, spawn};
use std::time::Duration;
use crate::handlers::path_handler;
use crate::Server;
use socketioxide::{SocketIo, TransportType};
use anyhow::Result;
use axum::routing::get;
use axum::Router;
use axum::Server as AxumServer;
use axum_util::cors::CorsLayer;
use serde_json::json;
use socketioxide::extract::SocketRef;
use tokio::runtime::Runtime;
use crate::actuator_handlers::ping_actuator;
use crate::actuator_methods::get_all_registered_actuators;
use crate::sensor_methods::{get_all_registered_sensors, get_all_last_sensor_readings};
use crate::events::{ALL_ACTUATORS_EVENT, ALL_LAST_SENSOR_READINGS_EVENT, ALL_SENSORS_EVENT, register_all_callbacks};
use crate::sensor_handlers::ping_sensor;


pub async fn run_socket_server() -> Result<SocketIo> {
    let address = env::var("SOCKET_IO_SERVER_ADDRESS").expect("SOCKET_IO_SERVER_ADDRESS must be set");

    let (layer, io) = SocketIo::builder()
        .connect_timeout(Duration::from_secs(30))
        .req_path("/socket.io")
        .transports([TransportType::Websocket, TransportType::Polling])
        .build_layer();

    io.ns("/", |socket: SocketRef| {
        println!("Socket connected : {:?}", socket.id);

        println!("Registering callbacks");

        register_all_callbacks(&socket);

        socket.on_disconnect(|socket: SocketRef| {
            println!("Socket disconnected : {:?}", socket.id);
        });

        match get_all_registered_sensors() {
            Ok(sensors) => {
                match socket.emit(
                    ALL_SENSORS_EVENT,
                    json!({
                            "sensors": sensors,
                        }),
                ) {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
            Err(_) => {}
        }

        match get_all_last_sensor_readings() {
            Ok(sensor_reads) => {
                match socket.emit(
                    ALL_LAST_SENSOR_READINGS_EVENT,
                    json!({
                            "sensor_reads": sensor_reads,
                        }),
                ) {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
            Err(_) => {}
        }

        match get_all_registered_actuators() {
            Ok(actuators) => {
                match socket.emit(
                    ALL_ACTUATORS_EVENT,
                    json!({
                            "actuators": actuators,
                        }),
                ) {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
            Err(_) => {}
        }
    });

    spawn(move || {
        Runtime::new()
            .expect("Failed to create Tokio runtime")
            .block_on(async move {
                let app = Router::new()
                    .route("/", get(|| async { "OK" }))
                    .layer(layer)
                    .layer(CorsLayer);

                println!("SocketIO server listening on {}", address);

                AxumServer::bind(&address.clone().parse::<SocketAddr>().expect("Failed to parse SocketIO server address"))
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

    spawn(move || {
        loop {
            match get_all_registered_sensors() {
                Ok(sensors) => {
                    sensors
                        .iter()
                        .for_each(|sensor| {
                            ping_sensor(sensor, boxed_socket.as_ref());
                        });
                }
                Err(_) => {}
            }

            match get_all_registered_actuators() {
                Ok(actuators) => {
                    actuators
                        .iter()
                        .for_each(|actuator| {
                            ping_actuator(actuator, boxed_socket.as_ref());
                        });
                }
                Err(_) => {}
            }

            std::thread::sleep(Duration::from_secs(5));
        }
    })
}

pub async fn run_coap_server(socket: &SocketIo) {
    let boxed_socket = Box::new(socket.clone());

    spawn(move || {
        println!("Starting CoAP server");

        Runtime::new()
            .expect("Failed to create Tokio runtime")
            .block_on(async {
                let address = env::var("COAP_SERVER_ADDRESS").expect("COAP_SERVER_ADDRESS must be set");

                let mut server = Server::new(address.clone()).unwrap();

                server.run(
                    |request| async {
                        let request_ref = &request;

                        let payload = path_handler(boxed_socket.as_ref(), &request_ref).await;

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
            });
    });
}