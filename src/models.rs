use diesel::{Identifiable, Insertable, Queryable, QueryableByName, Selectable};
use serde::{Serialize, Deserialize};

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