use anyhow::Result;
use diesel::prelude::*;
use crate::db::connect;
use crate::models::{NewSensor, Sensor};
use crate::schema::sensors::dsl::*;
use crate::schema::sensors;
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