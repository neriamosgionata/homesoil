// @generated automatically by Diesel CLI.

diesel::table! {
    actuators (id) {
        id -> Integer,
        name -> Nullable<Text>,
        ip_address -> Text,
        port -> SmallInt,
        state -> Bool,
        online -> Bool,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    sensor_reads (id) {
        id -> Integer,
        sensor_id -> Integer,
        sensor_value -> Text,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    sensors (id) {
        id -> Integer,
        name -> Nullable<Text>,
        sensor_type -> Text,
        ip_address -> Text,
        port -> SmallInt,
        online -> Bool,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::joinable!(sensor_reads -> sensors (sensor_id));

diesel::allow_tables_to_appear_in_same_query!(
    actuators,
    sensor_reads,
    sensors,
);
