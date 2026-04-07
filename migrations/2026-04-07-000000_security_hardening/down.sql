-- SQLite doesn't support DROP COLUMN directly, so we recreate the tables

-- Recreate sensors without dtls_supported
CREATE TABLE sensors_backup AS SELECT id, name, sensor_type, ip_address, port, online, created_at, updated_at FROM sensors;
DROP TABLE sensors;
CREATE TABLE sensors (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT DEFAULT 'Default',
    sensor_type TEXT NOT NULL,
    ip_address TEXT NOT NULL,
    port SMALLINT NOT NULL,
    online BOOLEAN NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME
);
INSERT INTO sensors SELECT * FROM sensors_backup;
DROP TABLE sensors_backup;
CREATE INDEX sensors_sensor_type_index ON sensors(sensor_type);

-- Recreate actuators without dtls_supported
CREATE TABLE actuators_backup AS SELECT id, name, ip_address, port, state, online, pulse, created_at, updated_at, intermittent, intermittent_on_ms, intermittent_off_ms FROM actuators;
DROP TABLE actuators;
CREATE TABLE actuators (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT DEFAULT 'Default',
    ip_address TEXT NOT NULL,
    port SMALLINT NOT NULL,
    state BOOLEAN NOT NULL DEFAULT 0,
    online BOOLEAN NOT NULL DEFAULT 0,
    pulse BOOLEAN NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL,
    updated_at DATETIME,
    intermittent BOOLEAN NOT NULL DEFAULT 0,
    intermittent_on_ms INTEGER NOT NULL DEFAULT 1000,
    intermittent_off_ms INTEGER NOT NULL DEFAULT 1000
);
INSERT INTO actuators SELECT * FROM actuators_backup;
DROP TABLE actuators_backup;
