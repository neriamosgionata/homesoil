use anyhow::Result;
use diesel::{insert_into, update};
use diesel::prelude::*;

use crate::db::connect;
use crate::models::{NewSensor, Sensor, SensorRead, NewSensorRead, UpdateSensorName, SensorUnregister};

use crate::schema::sensors::dsl::{id, ip_address, sensor_type, name};
use crate::schema::sensors;

use crate::schema::sensor_reads::dsl::{sensor_id, sensor_value};
use crate::schema::sensor_reads;
use serde_json::from_str;

pub fn register_sensor(payload: String) -> Result<Sensor> {
    println!("Registering sensor: {}", payload);

    let conn = &mut connect()?;

    let new_sensor = from_str::<NewSensor>(&payload)?;

    match sensors::table
        .filter(sensor_type.like(&new_sensor.get_sensor_type()))
        .filter(ip_address.like(&new_sensor.get_ip_address()))
        .get_result(conn)
    {
        Ok(sensor) => {
            println!("Sensor already registered: {:?}", sensor);
            return Ok(sensor);
        }
        Err(_) => {}
    }

    insert_into(sensors::table)
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

pub fn unregister_sensor(payload: String) -> Result<Sensor> {
    println!("Registering sensor: {}", payload);

    let conn = &mut connect()?;

    let new_sensor = from_str::<SensorUnregister>(&payload)?;

    let sensor = sensors::table
        .filter(id.eq(new_sensor.get_id()))
        .get_result::<Sensor>(conn)
        .expect("Error loading sensor");

    diesel::delete(sensors::table
        .filter(id.eq(new_sensor.get_id())))
        .execute(conn)
        .expect("Error deleting sensor");

    Ok(sensor)
}

pub fn change_sensor_name(payload: String) -> Result<Sensor> {
    println!("Changing sensor name: {}", payload);

    let conn = &mut connect()?;

    let update_sensor_name = from_str::<UpdateSensorName>(&payload)?;

    update(sensors::table.find(update_sensor_name.get_id()))
        .set(name.eq(update_sensor_name.get_name()))
        .execute(conn)
        .expect("Error updating sensor");

    let sensor = sensors::table
        .filter(id.eq(update_sensor_name.get_id()))
        .get_result(conn)
        .expect("Error loading sensor");

    Ok(sensor)
}

pub fn read_sensor(payload: String) -> Result<SensorRead> {
    println!("Reading sensor: {}", payload);

    let conn = &mut connect()?;

    let new_sensor_read = from_str::<NewSensorRead>(&payload)?;

    insert_into(sensor_reads::table)
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

pub fn get_all_registered_sensors() -> Result<Vec<Sensor>> {
    let conn = &mut connect()?;

    let sensors = sensors::table
        .get_results::<Sensor>(conn)
        .expect("Error loading sensors");

    Ok(sensors)
}

pub fn get_sensor_readings(other_sensor_id: i32) -> Result<Vec<SensorRead>> {
    let conn = &mut connect()?;

    let sensor_reads = sensor_reads::table
        .filter(sensor_id.eq(other_sensor_id))
        .get_results::<SensorRead>(conn)
        .expect("Error loading sensor reads");

    Ok(sensor_reads)
}