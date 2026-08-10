use crate::actuator_handlers::ping_actuator;
use crate::actuator_methods::get_all_registered_actuators;
use crate::auth::{
    check_rate_limit, clear_failed_attempts, generate_session_token, get_current_pin,
    is_pin_expired, is_valid_session_token, record_failed_attempt, rotate_pin,
    save_session_token,
};
use crate::events::{
    register_all_callbacks, ALL_ACTUATORS_EVENT, ALL_LAST_SENSOR_READINGS_EVENT,
    ALL_SENSORS_EVENT, ALL_FLOWS_EVENT,
};
use crate::helper::set_socket_io;
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
use std::collections::HashMap as StdHashMap;
use std::net::SocketAddr;
use std::sync::{Arc, LazyLock, Mutex as StdMutex};
use std::time::{Duration, Instant};

/// Rate limiter for CoAP multicast discovery: max 5 responses per source IP per 60 seconds.
static DISCOVERY_RATE_LIMITER: LazyLock<StdMutex<StdHashMap<String, (u32, Instant)>>> =
    LazyLock::new(|| StdMutex::new(StdHashMap::new()));

fn check_discovery_rate_limit(source_ip: &str) -> bool {
    let mut limiter = DISCOVERY_RATE_LIMITER.lock().unwrap();
    let now = Instant::now();

    if let Some((count, first_seen)) = limiter.get_mut(source_ip) {
        if now.duration_since(*first_seen) > Duration::from_secs(60) {
            // Window expired, reset
            *count = 1;
            *first_seen = now;
            true
        } else if *count >= 5 {
            false
        } else {
            *count += 1;
            true
        }
    } else {
        limiter.insert(source_ip.to_string(), (1, now));
        true
    }
}
use coap_lite::{MessageClass, ResponseType as Status};
use log::{info, warn};

#[derive(Debug, Deserialize)]
struct AuthData {
    #[serde(default)]
    token: String,
    #[serde(default)]
    pin: String,
}

pub async fn run_socket_server(address: &'static str) -> Result<SocketIo> {
    let (layer, io) = SocketIo::builder()
        .connect_timeout(Duration::from_secs(30))
        .req_path("/socket.io")
        .transports([TransportType::Websocket, TransportType::Polling])
        .build_layer();

    set_socket_io(io.clone());

    io.ns("/", move |socket: SocketRef, Data(auth): Data<AuthData>| {
        if !auth.pin.is_empty() && !check_rate_limit() {
            warn!("Rate limited, disconnecting socket");
            socket.disconnect().ok();
            return;
        }

        let authenticated = if !auth.token.is_empty() {
            is_valid_session_token(&auth.token)
        } else if !auth.pin.is_empty() {
            if is_pin_expired() {
                // Old PIN expired — rotate so pairing remains possible
                let new_pin = rotate_pin();
                warn!("PIN expired, rotated. New PIN: {}", new_pin);
                false
            } else {
                match get_current_pin() {
                    Some(current_pin) => auth.pin == current_pin,
                    None => false,
                }
            }
        } else {
            false
        };

        if !authenticated {
            if !auth.pin.is_empty() {
                record_failed_attempt();
                warn!("Invalid PIN attempt from socket: {:?}", socket.id);
            } else {
                warn!("Invalid credentials, disconnecting socket: {:?}", socket.id);
            }
            socket.disconnect().ok();
            return;
        }

        if !auth.pin.is_empty() {
            clear_failed_attempts();
            let session_token = generate_session_token();
            if save_session_token(&session_token) {
                let _: std::result::Result<_, _> =
                    socket.emit("session_token", json!({ "token": session_token }));
                info!("New device paired via PIN: {:?}", socket.id);
            }
        }

        info!("Socket connected: {:?}", socket.id);

        register_all_callbacks(&socket);

        socket.on_disconnect(|socket: SocketRef| {
            info!("Socket disconnected : {:?}", socket.id);
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

    tokio::spawn(async move {
        let app = Router::new()
            .route("/", get(|| async { "OK" }))
            .layer(layer)
            .layer(CorsLayer);

        let addr = address
            .parse::<SocketAddr>()
            .expect("Failed to parse SocketIO server address");

        let tls_cert = std::env::var("TLS_CERT_PATH").ok();
        let tls_key = std::env::var("TLS_KEY_PATH").ok();

        match (tls_cert, tls_key) {
            (Some(cert_path), Some(key_path)) => {
                info!("Starting SocketIO server with TLS on {}", address);
                let config = axum_server::tls_rustls::RustlsConfig::from_pem_file(
                    &cert_path, &key_path,
                )
                .await
                .expect("Failed to load TLS certificates");

                axum_server::bind_rustls(addr, config)
                    .serve(app.into_make_service())
                    .await
                    .unwrap();
            }
            _ => {
                warn!("TLS_CERT_PATH/TLS_KEY_PATH not set — running Socket.IO without TLS");
                info!("Starting SocketIO server on {}", address);

                AxumServer::bind(&addr)
                    .serve(app.into_make_service())
                    .await
                    .unwrap();
            }
        }

        info!("SocketIO server stopped");
    });

    Ok(io)
}

pub fn run_sensor_health_check(socket: &SocketIo) {
    let socket = socket.clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;

            let socket_clone = socket.clone();
            tokio::task::spawn_blocking(move || {
                if let Ok(sensors) = get_all_registered_sensors() {
                    sensors.iter().for_each(|sensor| {
                        ping_sensor(sensor, &socket_clone);
                    });
                }

                if let Ok(actuators) = get_all_registered_actuators() {
                    actuators.iter().for_each(|actuator| {
                        ping_actuator(actuator, &socket_clone);
                    });
                }
            }).await.ok();
        }
    });
}

pub async fn run_coap_server(
    bind_address: &'static str,
    actual_ip: &'static str,
    coap_port: u16,
    device_secret: &'static str,
    socket: &SocketIo,
) {
    let boxed_socket = Arc::new(socket.clone());

    tokio::spawn(async move {
        info!("Starting CoAP server on {}", bind_address);

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
                        // Rate limit discovery: max 5 per source IP per minute
                        let source_ip = request_ref
                            .source
                            .map(|s| s.ip().to_string())
                            .unwrap_or_default();

                        if !source_ip.is_empty()
                            && !check_discovery_rate_limit(&source_ip)
                        {
                            warn!(
                                "Discovery rate limited for {}",
                                source_ip
                            );
                            return None;
                        }

                        // If discovery is disabled via env, don't respond
                        if std::env::var("DISABLE_DISCOVERY")
                            .map(|v| v == "true" || v == "1")
                            .unwrap_or(false)
                        {
                            return None;
                        }

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
                                warn!(
                                    "Rejected CoAP request to {} — invalid device_secret",
                                    path
                                );
                                return match request.response {
                                    Some(mut message) => {
                                        message.message.header.code =
                                            MessageClass::Response(Status::Unauthorized);
                                        message.message.payload =
                                            b"Unauthorized".to_vec();
                                        Some(message)
                                    }
                                    _ => None,
                                };
                            }
                        } else {
                            warn!(
                                "Rejected CoAP request to {} — invalid payload",
                                path
                            );
                            return match request.response {
                                Some(mut message) => {
                                    message.message.header.code =
                                        MessageClass::Response(Status::BadRequest);
                                    message.message.payload =
                                        b"Bad Request".to_vec();
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
                                    message.message.header.code =
                                        MessageClass::Response(Status::NotFound);
                                    message.message.payload = b"Not Found".to_vec();
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
}

pub fn check_for_old_sensor_reads_records() {
    tokio::spawn(async {
        let mut interval = tokio::time::interval(Duration::from_secs(3600));
        loop {
            interval.tick().await;
            tokio::task::spawn_blocking(|| {
                let _: Result<_, _> = crate::sensor_methods::delete_old_sensor_reads_records();
            }).await.ok();
        }
    });
}
