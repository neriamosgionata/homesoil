use std::env;
use anyhow::{Error, Result};
use diesel::Connection;
use diesel::mysql::MysqlConnection;

pub fn connect() -> Result<MysqlConnection, Error> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    match MysqlConnection::establish(&database_url) {
        Ok(conn) => Ok(conn),
        Err(e) => Err(Error::new(e))
    }
}