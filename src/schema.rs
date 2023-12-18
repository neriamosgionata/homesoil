diesel::table! {
    sensors {
        id -> Integer,
        sensor_type -> VarChar,
        ip_address -> VarChar,
        name -> Nullable<VarChar>,
        online -> Bool,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    sensor_reads {
        id -> Integer,
        sensor_id -> Integer,
        sensor_value -> VarChar,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
    }
}


diesel::joinable!(sensor_reads -> sensors (sensor_id));

diesel::allow_tables_to_appear_in_same_query!(
    sensors,
    sensor_reads,
);

diesel::table! {
    actuators {
        id -> Integer,
        ip_address -> VarChar,
        name -> Nullable<VarChar>,
        online -> Bool,
        state -> Bool,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
    }
}
