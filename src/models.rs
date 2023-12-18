use diesel::{Identifiable, Insertable, Queryable, QueryableByName, Selectable};
use serde::{Serialize, Deserialize};


//SENSORS


#[derive(Debug, Clone, Queryable, Selectable, Deserialize, Serialize, PartialEq, Identifiable)]
#[diesel(table_name = crate::schema::sensors)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct Sensor {
    id: i32,
    sensor_type: String,
    ip_address: String,
    name: Option<String>,
    online: bool,
    created_at: chrono::NaiveDateTime,
    updated_at: Option<chrono::NaiveDateTime>,
}

impl Sensor {
    pub fn new(id: i32, sensor_type: &str, ip_address: &str) -> Self {
        Self {
            id,
            sensor_type: sensor_type.to_string(),
            ip_address: ip_address.to_string(),
            name: None,
            online: false,
            created_at: chrono::Local::now().naive_local(),
            updated_at: None,
        }
    }

    pub fn get_id(&self) -> i32 {
        self.id
    }

    pub fn get_sensor_type(&self) -> &str {
        &self.sensor_type
    }

    pub fn get_ip_address(&self) -> &str {
        &self.ip_address
    }

    pub fn get_name(&self) -> &Option<String> {
        &self.name
    }

    pub fn get_created_at(&self) -> &chrono::NaiveDateTime {
        &self.created_at
    }

    pub fn get_updated_at(&self) -> &Option<chrono::NaiveDateTime> {
        &self.updated_at
    }

    pub fn get_online(&self) -> bool {
        self.online
    }

    pub fn set_online(&mut self, online: bool) {
        self.online = online;
    }

    pub fn set_created_at(&mut self, created_at: chrono::NaiveDateTime) {
        self.created_at = created_at;
    }

    pub fn set_updated_at(&mut self, updated_at: chrono::NaiveDateTime) {
        self.updated_at = Some(updated_at);
    }

    pub fn set_name(&mut self, name: Option<String>) {
        self.name = name;
    }

    pub fn set_sensor_type(&mut self, sensor_type: String) {
        self.sensor_type = sensor_type;
    }

    pub fn set_ip_address(&mut self, ip_address: String) {
        self.ip_address = ip_address;
    }

    pub fn set_id(&mut self, id: i32) {
        self.id = id;
    }
}

#[derive(Debug, Clone, Queryable, Selectable, Deserialize, Serialize, PartialEq, Identifiable, QueryableByName)]
#[diesel(table_name = crate::schema::sensor_reads)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
#[diesel(belongs_to(Sensor))]
pub struct SensorRead {
    id: i32,
    sensor_id: i32,
    sensor_value: String,
    created_at: chrono::NaiveDateTime,
    updated_at: Option<chrono::NaiveDateTime>,
}

impl SensorRead {
    pub fn new(id: i32, sensor_id: i32, sensor_value: &str) -> Self {
        Self {
            id,
            sensor_id,
            sensor_value: sensor_value.to_string(),
            created_at: chrono::Local::now().naive_local(),
            updated_at: None,
        }
    }

    pub fn get_id(&self) -> i32 {
        self.id
    }

    pub fn get_sensor_id(&self) -> i32 {
        self.sensor_id
    }

    pub fn get_sensor_value(&self) -> &str {
        &self.sensor_value
    }

    pub fn get_created_at(&self) -> &chrono::NaiveDateTime {
        &self.created_at
    }

    pub fn get_updated_at(&self) -> &Option<chrono::NaiveDateTime> {
        &self.updated_at
    }

    pub fn set_created_at(&mut self, created_at: chrono::NaiveDateTime) {
        self.created_at = created_at;
    }

    pub fn set_updated_at(&mut self, updated_at: chrono::NaiveDateTime) {
        self.updated_at = Some(updated_at);
    }
}

//HELPERS

#[derive(Insertable, Deserialize, Serialize, Debug, Clone)]
#[diesel(table_name = crate::schema::sensors)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct NewSensor {
    sensor_type: String,
    ip_address: String,
    name: Option<String>,
    online: bool,
    created_at: Option<chrono::NaiveDateTime>,
}

impl NewSensor {
    pub fn new(sensor_type: &str, ip_address: &str) -> Self {
        Self {
            sensor_type: sensor_type.to_string(),
            ip_address: ip_address.to_string(),
            name: None,
            online: false,
            created_at: None,
        }
    }

    pub fn get_sensor_type(&self) -> &str {
        &self.sensor_type
    }

    pub fn get_ip_address(&self) -> &str {
        &self.ip_address
    }

    pub fn get_name(&self) -> &Option<String> {
        &self.name
    }

    pub fn get_created_at(&self) -> &Option<chrono::NaiveDateTime> {
        &self.created_at
    }

    pub fn set_created_at(&mut self, created_at: chrono::NaiveDateTime) {
        self.created_at = Some(created_at);
    }

    pub fn set_name(&mut self, name: Option<String>) {
        self.name = name;
    }

    pub fn set_sensor_type(&mut self, sensor_type: String) {
        self.sensor_type = sensor_type;
    }

    pub fn set_ip_address(&mut self, ip_address: String) {
        self.ip_address = ip_address;
    }

    pub fn set_online(&mut self, online: bool) {
        self.online = online;
    }

    pub fn get_online(&self) -> bool {
        self.online
    }
}

#[derive(Insertable, Deserialize, Serialize, Debug, Clone)]
#[diesel(table_name = crate::schema::sensor_reads)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct NewSensorRead {
    sensor_id: i32,
    sensor_value: String,
    created_at: Option<chrono::NaiveDateTime>,
}

impl NewSensorRead {
    pub fn new(sensor_id: i32, sensor_value: &str) -> Self {
        Self {
            sensor_id,
            sensor_value: sensor_value.to_string(),
            created_at: None,
        }
    }

    pub fn get_sensor_id(&self) -> i32 {
        self.sensor_id
    }

    pub fn get_sensor_value(&self) -> &str {
        &self.sensor_value
    }

    pub fn get_created_at(&self) -> &Option<chrono::NaiveDateTime> {
        &self.created_at
    }

    pub fn set_created_at(&mut self, created_at: chrono::NaiveDateTime) {
        self.created_at = Some(created_at);
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UpdateSensorName {
    id: i32,
    name: String,
    updated_at: Option<chrono::NaiveDateTime>,
}

impl UpdateSensorName {
    pub fn new(id: i32, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            updated_at: None,
        }
    }

    pub fn get_id(&self) -> i32 {
        self.id
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_updated_at(&self) -> &Option<chrono::NaiveDateTime> {
        &self.updated_at
    }

    pub fn set_updated_at(&mut self, updated_at: chrono::NaiveDateTime) {
        self.updated_at = Some(updated_at);
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SensorUnregister {
    id: i32,
}

impl SensorUnregister {
    pub fn new(id: i32) -> Self {
        Self { id }
    }

    pub fn get_id(&self) -> i32 {
        self.id
    }
}


//ACTUATORS

#[derive(Debug, Clone, Queryable, Selectable, Deserialize, Serialize, PartialEq, Identifiable)]
#[diesel(table_name = crate::schema::actuators)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct Actuator {
    id: i32,
    ip_address: String,
    name: Option<String>,
    online: bool,
    state: bool,
    created_at: chrono::NaiveDateTime,
    updated_at: Option<chrono::NaiveDateTime>,
}

impl Actuator {
    pub fn new(id: i32, ip_address: &str) -> Self {
        Self {
            id,
            ip_address: ip_address.to_string(),
            name: None,
            online: false,
            state: true,
            created_at: chrono::Local::now().naive_local(),
            updated_at: None,
        }
    }

    pub fn get_id(&self) -> i32 {
        self.id
    }

    pub fn get_ip_address(&self) -> &str {
        &self.ip_address
    }

    pub fn get_name(&self) -> &Option<String> {
        &self.name
    }

    pub fn get_created_at(&self) -> &chrono::NaiveDateTime {
        &self.created_at
    }

    pub fn get_updated_at(&self) -> &Option<chrono::NaiveDateTime> {
        &self.updated_at
    }

    pub fn get_online(&self) -> bool {
        self.online
    }

    pub fn set_online(&mut self, online: bool) {
        self.online = online;
    }

    pub fn set_created_at(&mut self, created_at: chrono::NaiveDateTime) {
        self.created_at = created_at;
    }

    pub fn set_updated_at(&mut self, updated_at: chrono::NaiveDateTime) {
        self.updated_at = Some(updated_at);
    }

    pub fn set_name(&mut self, name: Option<String>) {
        self.name = name;
    }

    pub fn set_ip_address(&mut self, ip_address: String) {
        self.ip_address = ip_address;
    }

    pub fn set_id(&mut self, id: i32) {
        self.id = id;
    }

    pub fn set_state(&mut self, state: bool) {
        self.state = state;
    }

    pub fn get_state(&self) -> bool {
        self.state
    }
}

//HELPERS

#[derive(Insertable, Deserialize, Serialize, Debug, Clone)]
#[diesel(table_name = crate::schema::actuators)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct NewActuator {
    ip_address: String,
    name: Option<String>,
    online: bool,
    state: bool,
    created_at: Option<chrono::NaiveDateTime>,
}

impl NewActuator {
    pub fn new(ip_address: &str) -> Self {
        Self {
            ip_address: ip_address.to_string(),
            name: None,
            online: false,
            state: false,
            created_at: None,
        }
    }

    pub fn get_ip_address(&self) -> &str {
        &self.ip_address
    }

    pub fn get_name(&self) -> &Option<String> {
        &self.name
    }

    pub fn get_created_at(&self) -> &Option<chrono::NaiveDateTime> {
        &self.created_at
    }

    pub fn set_created_at(&mut self, created_at: chrono::NaiveDateTime) {
        self.created_at = Some(created_at);
    }

    pub fn set_name(&mut self, name: Option<String>) {
        self.name = name;
    }

    pub fn set_ip_address(&mut self, ip_address: String) {
        self.ip_address = ip_address;
    }

    pub fn set_online(&mut self, online: bool) {
        self.online = online;
    }

    pub fn get_online(&self) -> bool {
        self.online
    }

    pub fn set_state(&mut self, state: bool) {
        self.state = state;
    }

    pub fn get_state(&self) -> bool {
        self.state
    }
}


#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UpdateActuatorName {
    id: i32,
    name: String,
    updated_at: Option<chrono::NaiveDateTime>,
}

impl UpdateActuatorName {
    pub fn new(id: i32, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            updated_at: None,
        }
    }

    pub fn get_id(&self) -> i32 {
        self.id
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_updated_at(&self) -> &Option<chrono::NaiveDateTime> {
        &self.updated_at
    }

    pub fn set_updated_at(&mut self, updated_at: chrono::NaiveDateTime) {
        self.updated_at = Some(updated_at);
    }
}


#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UpdateActuatorState {
    id: i32,
    state: bool,
    updated_at: Option<chrono::NaiveDateTime>,
}

impl UpdateActuatorState {
    pub fn new(id: i32, state: bool) -> Self {
        Self {
            id,
            state,
            updated_at: None,
        }
    }

    pub fn get_id(&self) -> i32 {
        self.id
    }

    pub fn get_state(&self) -> bool {
        self.state
    }

    pub fn get_updated_at(&self) -> &Option<chrono::NaiveDateTime> {
        &self.updated_at
    }

    pub fn set_updated_at(&mut self, updated_at: chrono::NaiveDateTime) {
        self.updated_at = Some(updated_at);
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ActuatorUnregister {
    id: i32,
}

impl ActuatorUnregister {
    pub fn new(id: i32) -> Self {
        Self { id }
    }

    pub fn get_id(&self) -> i32 {
        self.id
    }
}