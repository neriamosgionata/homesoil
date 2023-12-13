use std::net::SocketAddr;
use anyhow::Result;
use coap_lite::{CoapRequest, RequestType};
use diesel::prelude::*;

use crate::db::connect;
use crate::models::{NewSensor, Sensor, SensorRead, NewSensorRead};

use crate::schema::sensors::dsl::*;
use crate::schema::sensors;

use crate::schema::sensor_reads::dsl::*;
use crate::schema::sensor_reads;
use serde_json::from_str;

pub fn register_sensor(payload: String) -> Result<Sensor> {
    let conn = &mut connect()?;

    let new_sensor = from_str::<NewSensor>(&payload)?;

    diesel::insert_into(sensors::table)
        .values(&new_sensor)
        .execute(conn)
        .expect("Error saving new sensor");

    let sensor = sensors::table
        .filter(sensor_type.like(&new_sensor.get_sensor_type()))
        .filter(ip_address.like(&new_sensor.get_ip_address()))
        .get_result(conn)
        .expect("Error loading sensor");

    Ok(sensor)
}

pub fn read_sensor(payload: String) -> Result<SensorRead> {
    let conn = &mut connect()?;

    let new_sensor_read = from_str::<NewSensorRead>(&payload)?;

    diesel::insert_into(sensor_reads::table)
        .values(&new_sensor_read)
        .execute(conn)
        .expect("Error saving new sensor");

    let sensor_read = sensor_reads::table
        .filter(sensor_id.eq(new_sensor_read.get_sensor_id()))
        .filter(sensor_value.like(&new_sensor_read.get_sensor_value()))
        .get_result(conn)
        .expect("Error loading sensor");

    Ok(sensor_read)
}


pub fn sensor_register_handler() -> fn(&CoapRequest<SocketAddr>) -> String {
    |request: &CoapRequest<SocketAddr>| {
        let payload = String::from_utf8(request.message.payload.clone()).unwrap();

        if request.get_method() != &RequestType::Post {
            println!("Not a POST request");
            return "KO".to_string();
        }

        println!("POST request");

        match register_sensor(payload) {
            Ok(sensor) => {
                println!("Registered sensor: {:?}", sensor);
                sensor.get_id().to_string()
            }
            Err(e) => {
                println!("Error registering sensor: {:?}", e);
                "KO".to_string()
            }
        }
    }
}

pub fn sensor_read_handler() -> fn(&CoapRequest<SocketAddr>) -> String {
    |request: &CoapRequest<SocketAddr>| {
        let payload = String::from_utf8(request.message.payload.clone()).unwrap();

        if request.get_method() != &RequestType::Post {
            println!("Not a POST request");
            return "KO".to_string();
        }

        println!("POST request");

        match read_sensor(payload) {
            Ok(sensor_read) => {
                println!("Sensor read received: {:?}", sensor_read);
                "OK".to_string()
            }
            Err(e) => {
                println!("Error reading sensor: {:?}", e);
                "KO".to_string()
            }
        }
    }
}
