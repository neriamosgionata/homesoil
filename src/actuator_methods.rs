use anyhow::Result;
use diesel::{insert_into, update};
use diesel::prelude::*;

use crate::db::connect;
use crate::models::{NewActuator, UpdateActuatorName, SensorUnregister, Actuator, UpdateActuatorState};

use crate::schema::actuators::dsl::{id, ip_address, name};
use crate::schema::actuators;

use serde_json::from_str;
use crate::schema::actuators::state;

pub fn register_actuator(payload: String) -> Result<Actuator> {
    println!("Registering actuator: {}", payload);

    let conn = &mut connect()?;

    let mut new_actuator = from_str::<NewActuator>(&payload)?;

    new_actuator.set_created_at(chrono::Local::now().naive_local());

    match actuators::table
        .filter(ip_address.like(&new_actuator.get_ip_address()))
        .get_result(conn)
    {
        Ok(sensor) => {
            println!("actuator already registered: {:?}", sensor);
            return Ok(sensor);
        }
        Err(_) => {}
    }

    new_actuator.set_name(Some("Actuator".to_string()));

    insert_into(actuators::table)
        .values(&new_actuator)
        .execute(conn)
        .expect("Error saving new actuator");

    let sensor = actuators::table
        .filter(ip_address.like(&new_actuator.get_ip_address()))
        .get_result(conn)
        .expect("Error loading actuator");

    Ok(sensor)
}

pub fn unregister_actuator(payload: String) -> Result<Actuator> {
    println!("Registering actuator: {}", payload);

    let conn = &mut connect()?;

    let new_actuator = from_str::<SensorUnregister>(&payload)?;

    let sensor = actuators::table
        .filter(id.eq(new_actuator.get_id()))
        .get_result::<Actuator>(conn)
        .expect("Error loading actuator");

    diesel::delete(actuators::table
        .filter(id.eq(new_actuator.get_id())))
        .execute(conn)
        .expect("Error deleting actuator");

    Ok(sensor)
}

pub fn change_actuator_name(payload: String) -> Result<Actuator> {
    println!("Changing actuator name: {}", payload);

    let conn = &mut connect()?;

    let mut update_sensor_name = from_str::<UpdateActuatorName>(&payload)?;

    update_sensor_name.set_updated_at(chrono::Local::now().naive_local());

    update(actuators::table.find(update_sensor_name.get_id()))
        .set(name.eq(update_sensor_name.get_name()))
        .execute(conn)
        .expect("Error updating actuator");

    let sensor = actuators::table
        .filter(id.eq(update_sensor_name.get_id()))
        .get_result(conn)
        .expect("Error loading actuator");

    Ok(sensor)
}

pub fn change_actuator_state(payload: String) -> Result<Actuator> {
    println!("Changing actuator state: {}", payload);

    let conn = &mut connect()?;

    let mut update_actuator_state = from_str::<UpdateActuatorState>(&payload)?;

    update_actuator_state.set_updated_at(chrono::Local::now().naive_local());

    update(actuators::table.find(update_actuator_state.get_id()))
        .set(state.eq(update_actuator_state.get_state()))
        .execute(conn)
        .expect("Error updating actuator");

    let sensor = actuators::table
        .filter(id.eq(update_actuator_state.get_id()))
        .get_result(conn)
        .expect("Error loading actuator");

    Ok(sensor)
}

pub fn get_all_registered_actuators() -> Result<Vec<Actuator>> {
    let conn = &mut connect()?;

    let sensors = actuators::table
        .get_results::<Actuator>(conn)
        .expect("Error loading actuators");

    Ok(sensors)
}