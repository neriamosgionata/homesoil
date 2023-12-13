diesel::table! {
    sensors {
        id -> Integer,
        sensor_type -> VarChar,
        ip_address -> VarChar,
        name -> VarChar,
    }
}

diesel::table! {
    sensor_reads {
        id -> Integer,
        sensor_id -> Integer,
        sensor_value -> VarChar,
    }
}


diesel::joinable!(sensor_reads -> sensors (sensor_id));

diesel::allow_tables_to_appear_in_same_query!(
    sensors,
    sensor_reads,
);