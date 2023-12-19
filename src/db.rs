use std::env;
use anyhow::{Error, Result};
use diesel::Connection;
use diesel::sqlite::SqliteConnection;

pub fn connect() -> Result<SqliteConnection, Error> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    match SqliteConnection::establish(&database_url) {
        Ok(conn) => Ok(conn),
        Err(e) => Err(Error::new(e))
    }
}