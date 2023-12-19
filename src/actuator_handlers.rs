use std::net::SocketAddr;
use coap_lite::{CoapRequest, RequestType};
use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use serde_json::json;
use socketioxide::SocketIo;
use crate::actuator_methods::{change_actuator_name, change_actuator_state, register_actuator, unregister_actuator};
use crate::events::{ACTUATOR_NAME_CHANGE_EVENT, ACTUATOR_REGISTER_EVENT, ACTUATOR_STATE_CHANGE_EVENT, ACTUATOR_UNREGISTER_EVENT};

pub fn actuator_register_handler<'a>(socket: &'a SocketIo, request: &'a CoapRequest<SocketAddr>) -> BoxFuture<'a, String> {
    async move {
        let payload = String::from_utf8(request.message.payload.clone()).unwrap();

        if request.get_method() != &RequestType::Post {
            println!("Not a POST request");
            return "KO".to_string();
        }

        println!("POST request");

        match register_actuator(payload) {
            Ok(actuator) => {
                println!("Registered actuator: {:?}", actuator);

                match socket.of("/") {
                    Some(ns) => {
                        match ns.broadcast().emit(
                            ACTUATOR_REGISTER_EVENT,
                            json!({
                                    "actuator_id": actuator.get_id(),
                                    "actuator_name": actuator.get_name(),
                                    "actuator_ip_address": actuator.get_ip_address(),
                                    "actuator_port": actuator.get_port(),
                                    "actuator_pulse": actuator.get_pulse(),
                                    "online": actuator.get_online(),
                                    "created_at": actuator.get_created_at(),
                             }),
                        ) {
                            Ok(_) => {
                                println!("actuator register event emitted");
                            }
                            Err(e) => {
                                println!("Error emitting actuator register event: {:?}", e);
                            }
                        }
                    }
                    None => {}
                }

                json!({
                    "id": actuator.get_id(),
                    "state": actuator.get_state(),
                }).to_string()
            }
            Err(e) => {
                println!("Error registering actuator: {:?}", e);
                "KO".to_string()
            }
        }
    }
        .boxed()
}

pub fn actuator_unregister_handler<'a>(socket: &'a SocketIo, request: &'a CoapRequest<SocketAddr>) -> BoxFuture<'a, String> {
    async move {
        let payload = String::from_utf8(request.message.payload.clone()).unwrap();

        if request.get_method() != &RequestType::Post {
            println!("Not a POST request");
            return "KO".to_string();
        }

        println!("POST request");

        match unregister_actuator(payload) {
            Ok(actuator) => {
                println!("actuator unregistered: {:?}", actuator);

                match socket.of("/") {
                    Some(ns) => {
                        match ns.broadcast().emit(
                            ACTUATOR_UNREGISTER_EVENT,
                            json!({
                                    "actuator_id": actuator.get_id(),
                             }),
                        ) {
                            Ok(_) => {
                                println!("actuator unregister event emitted");
                            }
                            Err(e) => {
                                println!("Error emitting actuator unregister event: {:?}", e);
                            }
                        }
                    }
                    None => {}
                }

                actuator.get_id().to_string()
            }
            Err(e) => {
                println!("Error unregistering actuator: {:?}", e);
                "KO".to_string()
            }
        }
    }
        .boxed()
}

pub fn actuator_update_handler<'a>(socket: &'a SocketIo, request: &'a CoapRequest<SocketAddr>) -> BoxFuture<'a, String> {
    async move {
        if request.get_method() != &RequestType::Put {
            println!("Not a PUT request");
            return "KO".to_string();
        }

        println!("PUT request");

        let payload = String::from_utf8(request.message.payload.clone()).unwrap();

        match change_actuator_name(payload) {
            Ok(actuator) => {
                println!("actuator name changed: {:?}", actuator);

                match socket.of("/") {
                    Some(ns) => {
                        match ns.broadcast().emit(
                            ACTUATOR_NAME_CHANGE_EVENT,
                            json!({
                                    "actuator_id": actuator.get_id(),
                                    "actuator_name": actuator.get_name(),
                                    "updated_at": actuator.get_updated_at(),
                                }),
                        ) {
                            Ok(_) => {
                                println!("actuator name changed event emitted");
                            }
                            Err(e) => {
                                println!("Error emitting actuator name changed event: {:?}", e);
                            }
                        }
                    }
                    None => {}
                }


                "OK".to_string()
            }
            Err(e) => {
                println!("Error changing actuator name: {:?}", e);
                "KO".to_string()
            }
        }
    }
        .boxed()
}

pub fn actuator_update_state_handler<'a>(socket: &'a SocketIo, request: &'a CoapRequest<SocketAddr>) -> BoxFuture<'a, String> {
    async move {
        if request.get_method() != &RequestType::Put {
            println!("Not a PUT request");
            return "KO".to_string();
        }

        println!("PUT request");

        let payload = String::from_utf8(request.message.payload.clone()).unwrap();

        match change_actuator_state(payload) {
            Ok(actuator) => {
                println!("actuator state changed: {:?}", actuator);

                match socket.of("/") {
                    Some(ns) => {
                        match ns.broadcast().emit(
                            ACTUATOR_STATE_CHANGE_EVENT,
                            json!({
                                    "actuator_id": actuator.get_id(),
                                    "actuator_state": actuator.get_state(),
                                    "updated_at": actuator.get_updated_at(),
                                }),
                        ) {
                            Ok(_) => {
                                println!("actuator state changed event emitted");
                            }
                            Err(e) => {
                                println!("Error emitting actuator state changed event: {:?}", e);
                            }
                        }
                    }
                    None => {}
                }


                "OK".to_string()
            }
            Err(e) => {
                println!("Error changing actuator state: {:?}", e);
                "KO".to_string()
            }
        }
    }
        .boxed()
}