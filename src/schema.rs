diesel::table! {
    sensors {
        id -> Integer,
        sensor_type -> VarChar,
        ip_address -> VarChar,
    }
}