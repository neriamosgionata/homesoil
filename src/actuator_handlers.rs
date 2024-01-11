use std::net::SocketAddr;
use coap_lite::{CoapRequest, RequestType};
use diesel::prelude::*;
use diesel::update;
use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use serde_json::json;
use socketioxide::SocketIo;
use crate::actuator_methods::{change_actuator_name, change_actuator_state, register_actuator, unregister_actuator};
use crate::CoAPClient;
use crate::db::connect;
use crate::events::{ACTUATOR_CHANGE_ONLINE_EVENT, ACTUATOR_NAME_CHANGE_EVENT, ACTUATOR_REGISTER_EVENT, ACTUATOR_STATE_CHANGE_EVENT, ACTUATOR_UNREGISTER_EVENT};
use crate::models::{Actuator};
use crate::schema::actuators;
use crate::schema::actuators::{online, updated_at};
use anyhow::{Result};

pub fn actuator_register_handler<'a>(socket: &'a SocketIo, request: &'a CoapRequest<SocketAddr>) -> BoxFuture<'a, String> {
    async move {
        let payload = String::from_utf8(request.message.payload.clone()).unwrap();

        if request.get_method() != &RequestType::Post {
            return "KO".to_string();
        }

        match register_actuator(payload) {
            Ok(actuator) => {
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
                            Ok(_) => {}
                            Err(e) => {
                                println!("Error emitting actuator register event: {:?}", e);
                            }
                        }
                    }
                    None => {}
                }

                match socket.of("/") {
                    Some(ns) => {
                        match ns.emit(
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
                            Ok(_) => {}
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
            return "KO".to_string();
        }

        match unregister_actuator(payload) {
            Ok(actuator) => {
                match socket.of("/") {
                    Some(ns) => {
                        match ns.broadcast().emit(
                            ACTUATOR_UNREGISTER_EVENT,
                            json!({
                                    "actuator_id": actuator.get_id(),
                             }),
                        ) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Error emitting actuator unregister event broadcast: {:?}", e);
                            }
                        }
                    }
                    None => {}
                }

                match socket.of("/") {
                    Some(ns) => {
                        match ns.emit(
                            ACTUATOR_UNREGISTER_EVENT,
                            json!({
                                    "actuator_id": actuator.get_id(),
                             }),
                        ) {
                            Ok(_) => {}
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
            return "KO".to_string();
        }

        let payload = String::from_utf8(request.message.payload.clone()).unwrap();

        match change_actuator_name(payload) {
            Ok(actuator) => {
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
                            Ok(_) => {}
                            Err(e) => {
                                println!("Error emitting actuator name changed event broadcast: {:?}", e);
                            }
                        }
                    }
                    None => {}
                }

                match socket.of("/") {
                    Some(ns) => {
                        match ns.emit(
                            ACTUATOR_NAME_CHANGE_EVENT,
                            json!({
                                    "actuator_id": actuator.get_id(),
                                    "actuator_name": actuator.get_name(),
                                    "updated_at": actuator.get_updated_at(),
                                }),
                        ) {
                            Ok(_) => {}
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
            return "KO".to_string();
        }

        let payload = String::from_utf8(request.message.payload.clone()).unwrap();

        match change_actuator_state(payload) {
            Ok(actuator) => {
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
                            Ok(_) => {}
                            Err(e) => {
                                println!("Error emitting actuator state changed event broadcast: {:?}", e);
                            }
                        }
                    }
                    None => {}
                }

                match socket.of("/") {
                    Some(ns) => {
                        match ns.emit(
                            ACTUATOR_STATE_CHANGE_EVENT,
                            json!({
                                    "actuator_id": actuator.get_id(),
                                    "actuator_state": actuator.get_state(),
                                    "updated_at": actuator.get_updated_at(),
                                }),
                        ) {
                            Ok(_) => {}
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

pub fn ping_actuator(actuator: &Actuator, socket: &SocketIo) {
    let address = "coap://".to_owned() + actuator.get_ip_address() + ":" + actuator.get_port().to_string().as_str();

    match CoAPClient::get(&address) {
        Ok(_) => {
            if !actuator.get_online() {
                let conn = &mut match connect() {
                    Ok(conn) => conn,
                    Err(_) => {
                        return;
                    }
                };

                let uat = chrono::Local::now().naive_local();

                let res = update(actuators::table.find(actuator.get_id()))
                    .set((online.eq(true), updated_at.eq(uat)))
                    .execute(conn);

                match res {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error updating actuator: {:?}", e);
                        return;
                    }
                }

                match socket.of("/") {
                    Some(ns) => {
                        match ns.broadcast().emit(
                            ACTUATOR_CHANGE_ONLINE_EVENT,
                            json!({
                                    "actuator_id": actuator.get_id(),
                                    "online": true,
                                    "updated_at": uat,
                             }),
                        ) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Error emitting actuator online event broadcast: {:?}", e);
                            }
                        }
                    }
                    None => {}
                }

                match socket.of("/") {
                    Some(ns) => {
                        match ns.emit(
                            ACTUATOR_CHANGE_ONLINE_EVENT,
                            json!({
                                    "actuator_id": actuator.get_id(),
                                    "online": true,
                                    "updated_at": uat,
                             }),
                        ) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Error emitting actuator online event: {:?}", e);
                            }
                        }
                    }
                    None => {}
                }
            }
        }
        Err(_) => {
            let conn = &mut connect().unwrap();

            let uat = chrono::Local::now().naive_local();

            let res = update(actuators::table.find(actuator.get_id()))
                .set((online.eq(false), updated_at.eq(uat)))
                .execute(conn);

            match res {
                Ok(_) => {}
                Err(e) => {
                    println!("Error updating actuator: {:?}", e);
                    return;
                }
            }

            match socket.of("/") {
                Some(ns) => {
                    match ns.broadcast().emit(
                        ACTUATOR_CHANGE_ONLINE_EVENT,
                        json!({
                                    "actuator_id": actuator.get_id(),
                                    "online": true,
                                    "updated_at": uat,
                             }),
                    ) {
                        Ok(_) => {}
                        Err(e) => {
                            println!("Error emitting actuator online event broadcast: {:?}", e);
                        }
                    }
                }
                None => {}
            }

            match socket.of("/") {
                Some(ns) => {
                    match ns.emit(
                        ACTUATOR_CHANGE_ONLINE_EVENT,
                        json!({
                                    "actuator_id": actuator.get_id(),
                                    "online": true,
                                    "updated_at": uat,
                             }),
                    ) {
                        Ok(_) => {}
                        Err(e) => {
                            println!("Error emitting actuator online event: {:?}", e);
                        }
                    }
                }
                None => {}
            }
        }
    };
}

pub fn send_message_to_actuator(actuator_id: i32, message: &String) -> Result<String> {
    let conn = &mut connect()?;

    let actuator = actuators::table
        .filter(actuators::id.eq(actuator_id))
        .get_result::<Actuator>(conn);

    if actuator.is_err() {
        return Ok("KO".to_string());
    }

    let actuator = actuator.unwrap();

    let address = "coap://".to_owned() + actuator.get_ip_address() + ":" + actuator.get_port().to_string().as_str();

    match CoAPClient::post(&address, message.as_bytes().to_vec()) {
        Ok(res) => {
            let payload = String::from_utf8(res.message.payload);

            match payload {
                Ok(payload) => {
                    Ok(payload)
                }
                Err(_) => {
                    Ok("KO".to_string())
                }
            }
        }
        Err(_) => {
            Ok("KO".to_string())
        }
    }
}