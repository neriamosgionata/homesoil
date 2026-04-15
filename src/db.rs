use anyhow::{Error, Result};
use diesel::r2d2::{self, ConnectionManager, Pool, PooledConnection};
use diesel::sqlite::SqliteConnection;
use std::env;
use std::sync::OnceLock;

type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;

static POOL: OnceLock<SqlitePool> = OnceLock::new();

/// Initialize the global connection pool. Call once at startup.
pub fn init_pool() {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<SqliteConnection>::new(&database_url);

    let pool = r2d2::Pool::builder()
        .max_size(4)
        .min_idle(Some(1))
        .connection_customizer(Box::new(SqlitePragmas))
        .build(manager)
        .expect("Failed to create connection pool");

    POOL.set(pool).expect("Pool already initialized");
}

/// Get a connection from the pool.
pub fn connect() -> Result<PooledConnection<ConnectionManager<SqliteConnection>>, Error> {
    let pool = POOL.get().expect("Pool not initialized — call init_pool() first");
    pool.get().map_err(|e| Error::msg(e.to_string()))
}

#[derive(Debug)]
struct SqlitePragmas;

impl r2d2::CustomizeConnection<SqliteConnection, r2d2::Error> for SqlitePragmas {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> std::result::Result<(), r2d2::Error> {
        use diesel::RunQueryDsl;
        diesel::sql_query("PRAGMA journal_mode = WAL").execute(conn).map_err(r2d2::Error::QueryError)?;
        diesel::sql_query("PRAGMA synchronous = NORMAL").execute(conn).map_err(r2d2::Error::QueryError)?;
        diesel::sql_query("PRAGMA busy_timeout = 5000").execute(conn).map_err(r2d2::Error::QueryError)?;
        diesel::sql_query("PRAGMA cache_size = -2000").execute(conn).map_err(r2d2::Error::QueryError)?;
        diesel::sql_query("PRAGMA temp_store = MEMORY").execute(conn).map_err(r2d2::Error::QueryError)?;
        Ok(())
    }
}

