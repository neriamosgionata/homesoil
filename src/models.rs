use diesel::{Insertable, Queryable, Selectable};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Queryable, Selectable, Deserialize, Serialize)]
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
