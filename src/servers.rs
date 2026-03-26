use crate::actuator_handlers::ping_actuator;
use crate::actuator_methods::get_all_registered_actuators;
use crate::auth::{
    check_rate_limit, clear_failed_attempts, generate_session_token, is_valid_session_token,
    record_failed_attempt, save_session_token,
};
use crate::events::{
    register_all_callbacks, ALL_ACTUATORS_EVENT, ALL_LAST_SENSOR_READINGS_EVENT,
    ALL_SENSORS_EVENT, ALL_FLOWS_EVENT,
};
use crate::intermittent::set_socket_io;
use crate::flow_methods::get_flows;
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
    #[serde(default)]
    token: String,
    #[serde(default)]
    pin: String,
}

pub async fn run_socket_server(address: &'static str, pin: &'static str) -> Result<SocketIo> {
    let (layer, io) = SocketIo::builder()
        .connect_timeout(Duration::from_secs(30))
        .req_path("/socket.io")
        .transports([TransportType::Websocket, TransportType::Polling])
        .build_layer();

    set_socket_io(io.clone());

    io.ns("/", move |socket: SocketRef, Data(auth): Data<AuthData>| {
        let socket_id = format!("{:?}", socket.id);

        if !auth.pin.is_empty() && !check_rate_limit(&socket_id) {
            println!("Rate limited, disconnecting socket: {}", socket_id);
            socket.disconnect().ok();
            return;
        }

        let authenticated = if !auth.token.is_empty() {
            is_valid_session_token(&auth.token)
        } else if !auth.pin.is_empty() {
            auth.pin == pin
        } else {
            false
        };

        if !authenticated {
            if !auth.pin.is_empty() {
                record_failed_attempt(&socket_id);
                println!("Invalid PIN attempt from socket: {}", socket_id);
            } else {
                println!("Invalid credentials, disconnecting socket: {}", socket_id);
            }
            socket.disconnect().ok();
            return;
        }

        if !auth.pin.is_empty() {
            clear_failed_attempts(&socket_id);
            let session_token = generate_session_token();
            if save_session_token(&session_token) {
                let _: std::result::Result<_, _> =
                    socket.emit("session_token", json!({ "token": session_token }));
                println!("New device paired via PIN: {}", socket_id);
            }
        }

        println!("Socket connected: {}", socket_id);

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

        if let Ok(flows) = get_flows() {
            let _: Result<_, _> = socket.emit(
                ALL_FLOWS_EVENT,
                json!({
                    "flows": flows,
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

pub async fn run_coap_server(
    bind_address: &'static str,
    actual_ip: &'static str,
    coap_port: u16,
    device_secret: &'static str,
    socket: &SocketIo,
) {
    let boxed_socket = Arc::new(socket.clone());

    spawn(move || {
        println!("Starting CoAP server on {}", bind_address);

        Runtime::new()
            .expect("Failed to create Tokio runtime")
            .block_on(async {
                let mut server = Server::new(bind_address).unwrap();
                server.enable_all_coap(0);

                let discovery_payload = format!(
                    r#"{{"ip":"{}","coap_port":{}}}"#,
                    actual_ip, coap_port
                );

                server
                    .run(move |request| {
                        let discovery_payload = discovery_payload.clone();
                        let boxed_socket = boxed_socket.clone();
                        async move {
                            let request_ref = &request;

                            let path = format!("{}{}", "/", request_ref.get_path());
                            if path == "/.well-known/core" {
                                return match request.response {
                                    Some(mut message) => {
                                        message.message.payload =
                                            discovery_payload.as_bytes().to_vec();
                                        Some(message)
                                    }
                                    _ => None,
                                };
                            }

                            // Validate device secret from JSON payload
                            if let Ok(body) =
                                String::from_utf8(request_ref.message.payload.clone())
                            {
                                if let Ok(parsed) =
                                    serde_json::from_str::<serde_json::Value>(&body)
                                {
                                    let secret = parsed
                                        .get("device_secret")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");
                                    if secret != device_secret {
                                        println!(
                                            "Rejected CoAP request to {} — invalid device_secret",
                                            path
                                        );
                                        return match request.response {
                                            Some(mut message) => {
                                                message.message.payload =
                                                    b"Unauthorized".to_vec();
                                                Some(message)
                                            }
                                            _ => None,
                                        };
                                    }
                                } else {
                                    println!(
                                        "Rejected CoAP request to {} — invalid payload",
                                        path
                                    );
                                    return match request.response {
                                        Some(mut message) => {
                                            message.message.payload =
                                                b"Unauthorized".to_vec();
                                            Some(message)
                                        }
                                        _ => None,
                                    };
                                }
                            }

                            let payload =
                                path_handler(boxed_socket.as_ref(), request_ref).await;

                            match request.response {
                                Some(mut message) => {
                                    match payload {
                                        Some(payload) => {
                                            message.message.payload =
                                                payload.as_bytes().to_vec();
                                        }
                                        None => {
                                            message.message.payload = b"Error".to_vec();
                                        }
                                    }
                                    Some(message)
                                }
                                _ => None,
                            }
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
