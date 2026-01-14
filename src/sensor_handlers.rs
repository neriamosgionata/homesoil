use crate::db::connect;
use crate::events::{
    SENSOR_CHANGE_ONLINE_EVENT, SENSOR_NAME_CHANGE_EVENT, SENSOR_READ_EVENT, SENSOR_REGISTER_EVENT,
    SENSOR_UNREGISTER_EVENT,
};
use crate::models::Sensor;
use crate::schema::sensors;
use crate::schema::sensors::{online, updated_at};
use crate::sensor_methods::{change_sensor_name, read_sensor, register_sensor, unregister_sensor};
use crate::CoAPClient;
use anyhow::{Error, Result};
use coap_lite::{CoapRequest, RequestType};
use diesel::prelude::*;
use diesel::{update, QueryDsl};
use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use serde_json::json;
use socketioxide::SocketIo;
use std::net::SocketAddr;

pub fn sensor_register_handler<'a>(
    socket: &'a SocketIo,
    request: &'a CoapRequest<SocketAddr>,
) -> BoxFuture<'a, String> {
    async move {
        let payload = match String::from_utf8(request.message.payload.clone()) {
            Ok(p) => p,
            Err(_) => return "KO".to_string(),
        };

        if request.get_method() != &RequestType::Post {
            return "KO".to_string();
        }

        match register_sensor(payload) {
            Ok(sensor) => {
                if let Some(ns) = socket.of("/") {
                    match ns.broadcast().emit(
                        SENSOR_REGISTER_EVENT,
                        json!({
                               "sensor_id": sensor.get_id(),
                               "sensor_name": sensor.get_name(),
                               "sensor_ip_address": sensor.get_ip_address(),
                               "sensor_port": sensor.get_port(),
                               "sensor_type": sensor.get_sensor_type(),
                               "online": sensor.get_online(),
                               "created_at": sensor.get_created_at(),
                        }),
                    ) {
                        Ok(_) => {}
                        Err(e) => {
                            println!("Error emitting sensor register event: {:?}", e);
                        }
                    }
                }

                if let Some(ns) = socket.of("/") {
                    match ns.emit(
                        SENSOR_REGISTER_EVENT,
                        json!({
                               "sensor_id": sensor.get_id(),
                               "sensor_name": sensor.get_name(),
                               "sensor_ip_address": sensor.get_ip_address(),
                               "sensor_port": sensor.get_port(),
                               "sensor_type": sensor.get_sensor_type(),
                               "online": sensor.get_online(),
                               "created_at": sensor.get_created_at(),
                        }),
                    ) {
                        Ok(_) => {}
                        Err(e) => {
                            println!("Error emitting sensor register event: {:?}", e);
                        }
                    }
                }

                sensor.get_id().to_string()
            }
            Err(e) => {
                println!("Error registering sensor: {:?}", e);
                "KO".to_string()
            }
        }
    }
    .boxed()
}

pub fn sensor_unregister_handler<'a>(
    socket: &'a SocketIo,
    request: &'a CoapRequest<SocketAddr>,
) -> BoxFuture<'a, String> {
    async move {
        let payload = match String::from_utf8(request.message.payload.clone()) {
            Ok(p) => p,
            Err(_) => return "KO".to_string(),
        };

        if request.get_method() != &RequestType::Post {
            return "KO".to_string();
        }

        match unregister_sensor(payload) {
            Ok(sensor) => {
                if let Some(ns) = socket.of("/") {
                    match ns.broadcast().emit(
                        SENSOR_UNREGISTER_EVENT,
                        json!({
                               "sensor_id": sensor.get_id(),
                        }),
                    ) {
                        Ok(_) => {}
                        Err(e) => {
                            println!("Error emitting sensor unregister event: {:?}", e);
                        }
                    }
                }

                if let Some(ns) = socket.of("/") {
                    match ns.emit(
                        SENSOR_UNREGISTER_EVENT,
                        json!({
                               "sensor_id": sensor.get_id(),
                        }),
                    ) {
                        Ok(_) => {}
                        Err(e) => {
                            println!("Error emitting sensor unregister event: {:?}", e);
                        }
                    }
                }

                sensor.get_id().to_string()
            }
            Err(e) => {
                println!("Error unregistering sensor: {:?}", e);
                "KO".to_string()
            }
        }
    }
    .boxed()
}

pub fn sensor_read_handler<'a>(
    socket: &'a SocketIo,
    request: &'a CoapRequest<SocketAddr>,
) -> BoxFuture<'a, String> {
    async move {
        let payload = match String::from_utf8(request.message.payload.clone()) {
            Ok(p) => p,
            Err(_) => return "KO".to_string(),
        };

        if request.get_method() != &RequestType::Post {
            return "KO".to_string();
        }

        match read_sensor(payload) {
            Ok(sensor_read) => {
                if let Some(ns) = socket.of("/") {
                    match ns.broadcast().emit(
                        SENSOR_READ_EVENT,
                        json!({
                            "id": sensor_read.get_id(),
                            "sensor_id": sensor_read.get_sensor_id(),
                            "sensor_value": sensor_read.get_sensor_value(),
                            "created_at": sensor_read.get_created_at(),
                        }),
                    ) {
                        Ok(_) => {}
                        Err(e) => {
                            println!("Error emitting sensor read event broadcast: {:?}", e);
                        }
                    }
                }

                if let Some(ns) = socket.of("/") {
                    match ns.emit(
                        SENSOR_READ_EVENT,
                        json!({
                            "id": sensor_read.get_id(),
                            "sensor_id": sensor_read.get_sensor_id(),
                            "sensor_value": sensor_read.get_sensor_value(),
                            "created_at": sensor_read.get_created_at(),
                        }),
                    ) {
                        Ok(_) => {}
                        Err(e) => {
                            println!("Error emitting sensor read event: {:?}", e);
                        }
                    }
                }

                "OK".to_string()
            }
            Err(e) => {
                println!("Error reading sensor: {:?}", e);
                "KO".to_string()
            }
        }
    }
    .boxed()
}

pub fn sensor_update_handler<'a>(
    socket: &'a SocketIo,
    request: &'a CoapRequest<SocketAddr>,
) -> BoxFuture<'a, String> {
    async move {
        if request.get_method() != &RequestType::Put {
            return "KO".to_string();
        }

        let payload = match String::from_utf8(request.message.payload.clone()) {
            Ok(p) => p,
            Err(_) => return "KO".to_string(),
        };

        match change_sensor_name(payload) {
            Ok(sensor) => {
                if let Some(ns) = socket.of("/") {
                    match ns.broadcast().emit(
                        SENSOR_NAME_CHANGE_EVENT,
                        json!({
                            "sensor_id": sensor.get_id(),
                            "sensor_name": sensor.get_name(),
                            "updated_at": sensor.get_updated_at(),
                        }),
                    ) {
                        Ok(_) => {}
                        Err(e) => {
                            println!(
                                "Error emitting sensor name changed event broadcast: {:?}",
                                e
                            );
                        }
                    }
                }

                if let Some(ns) = socket.of("/") {
                    match ns.emit(
                        SENSOR_NAME_CHANGE_EVENT,
                        json!({
                            "sensor_id": sensor.get_id(),
                            "sensor_name": sensor.get_name(),
                            "updated_at": sensor.get_updated_at(),
                        }),
                    ) {
                        Ok(_) => {}
                        Err(e) => {
                            println!("Error emitting sensor name changed event: {:?}", e);
                        }
                    }
                }

                "OK".to_string()
            }
            Err(e) => {
                println!("Error changing sensor name: {:?}", e);
                "KO".to_string()
            }
        }
    }
    .boxed()
}

pub fn ping_sensor(sensor: &Sensor, socket: &SocketIo) {
    let address = "coap://".to_owned()
        + sensor.get_ip_address()
        + ":"
        + sensor.get_port().to_string().as_str();

    match CoAPClient::get(&address) {
        Ok(_) => {
            let conn = &mut match connect() {
                Ok(conn) => conn,
                Err(_) => {
                    return;
                }
            };

            let uat = chrono::Local::now().naive_local();

            let res = update(sensors::table.find(sensor.get_id()))
                .set((online.eq(true), updated_at.eq(uat)))
                .execute(conn);

            match res {
                Ok(_) => {}
                Err(e) => {
                    println!("Error updating sensor: {:?}", e);
                    return;
                }
            }

            if let Some(ns) = socket.of("/") {
                match ns.broadcast().emit(
                    SENSOR_CHANGE_ONLINE_EVENT,
                    json!({
                           "sensor_id": sensor.get_id(),
                           "online": true,
                           "updated_at": uat,
                    }),
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error emitting sensor online event broadcast: {:?}", e);
                    }
                }
            }

            if let Some(ns) = socket.of("/") {
                match ns.emit(
                    SENSOR_CHANGE_ONLINE_EVENT,
                    json!({
                           "sensor_id": sensor.get_id(),
                           "online": true,
                           "updated_at": uat,
                    }),
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error emitting sensor online event: {:?}", e);
                    }
                }
            }
        }
        Err(_) => {
            let conn = &mut match connect() {
                Ok(conn) => conn,
                Err(_) => {
                    return;
                }
            };

            let uat = chrono::Local::now().naive_local();

            let res = update(sensors::table.find(sensor.get_id()))
                .set((online.eq(false), updated_at.eq(uat)))
                .execute(conn);

            match res {
                Ok(_) => {}
                Err(e) => {
                    println!("Error updating sensor: {:?}", e);
                    return;
                }
            }

            if let Some(ns) = socket.of("/") {
                match ns.broadcast().emit(
                    SENSOR_CHANGE_ONLINE_EVENT,
                    json!({
                           "sensor_id": sensor.get_id(),
                           "online": false,
                           "updated_at": uat,
                    }),
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error emitting sensor online event broadcast: {:?}", e);
                    }
                }
            }

            if let Some(ns) = socket.of("/") {
                match ns.emit(
                    SENSOR_CHANGE_ONLINE_EVENT,
                    json!({
                           "sensor_id": sensor.get_id(),
                           "online": false,
                           "updated_at": uat,
                    }),
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error emitting sensor online event: {:?}", e);
                    }
                }
            }
        }
    };
}

pub fn send_message_to_sensor(sensor_id: i32, message: &String) -> Result<String> {
    let conn = &mut connect()?;

    let sensor = match sensors::table
        .filter(sensors::id.eq(sensor_id))
        .get_result::<Sensor>(conn)
    {
        Ok(sensor) => sensor,
        Err(_) => {
            return Err(Error::msg("Error loading sensor"));
        }
    };

    let address = "coap://".to_owned()
        + sensor.get_ip_address()
        + ":"
        + sensor.get_port().to_string().as_str();

    match CoAPClient::post(&address, message.as_bytes().to_vec()) {
        Ok(res) => {
            let payload = String::from_utf8(res.message.payload);

            match payload {
                Ok(payload) => Ok(payload),
                Err(_) => Err(Error::msg("Error parsing payload")),
            }
        }
        Err(_) => Err(Error::msg("Error sending message to sensor")),
    }
}

