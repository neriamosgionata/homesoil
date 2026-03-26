-- SQLite doesn't support DROP COLUMN before 3.35.0, recreate table
CREATE TABLE actuators_backup (
    id INTEGER PRIMARY KEY NOT NULL,
    name TEXT,
    ip_address TEXT NOT NULL,
    port SMALLINT NOT NULL,
    state BOOLEAN NOT NULL DEFAULT FALSE,
    online BOOLEAN NOT NULL DEFAULT FALSE,
    pulse BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP
);

INSERT INTO actuators_backup SELECT id, name, ip_address, port, state, online, pulse, created_at, updated_at FROM actuators;
DROP TABLE actuators;
ALTER TABLE actuators_backup RENAME TO actuators;
