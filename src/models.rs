use diesel::{Identifiable, Insertable, Queryable, Selectable};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Queryable, Selectable, Deserialize, Serialize, PartialEq, Identifiable)]
#[diesel(table_name = crate::schema::sensors)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct Sensor {
    id: i32,
    sensor_type: String,
    ip_address: String,
}

impl Sensor {
    pub fn new(id: i32, sensor_type: &str, ip_address: &str) -> Self {
        Self {
            id,
            sensor_type: sensor_type.to_string(),
            ip_address: ip_address.to_string(),
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
}

#[derive(Insertable, Deserialize, Serialize, Debug, Clone)]
#[diesel(table_name = crate::schema::sensors)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct NewSensor {
    sensor_type: String,
    ip_address: String,
}

impl NewSensor {
    pub fn new(sensor_type: &str, ip_address: &str) -> Self {
        Self {
            sensor_type: sensor_type.to_string(),
            ip_address: ip_address.to_string(),
        }
    }

    pub fn get_sensor_type(&self) -> &str {
        &self.sensor_type
    }

    pub fn get_ip_address(&self) -> &str {
        &self.ip_address
    }
}

#[derive(Debug, Clone, Queryable, Selectable, Deserialize, Serialize, PartialEq, Identifiable)]
#[diesel(table_name = crate::schema::sensor_reads)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
#[diesel(belongs_to(Sensor))]
pub struct SensorRead {
    id: i32,
    sensor_id: i32,
    sensor_value: String,
}

impl SensorRead {
    pub fn new(id: i32, sensor_id: i32, sensor_value: &str) -> Self {
        Self {
            id,
            sensor_id,
            sensor_value: sensor_value.to_string(),
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
}

#[derive(Insertable, Deserialize, Serialize, Debug, Clone)]
#[diesel(table_name = crate::schema::sensor_reads)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct NewSensorRead {
    sensor_id: i32,
    sensor_value: String,
}

impl NewSensorRead {
    pub fn new(sensor_id: i32, sensor_value: &str) -> Self {
        Self {
            sensor_id,
            sensor_value: sensor_value.to_string(),
        }
    }

    pub fn get_sensor_id(&self) -> i32 {
        self.sensor_id
    }

    pub fn get_sensor_value(&self) -> &str {
        &self.sensor_value
    }
}
