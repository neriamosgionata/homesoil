CREATE TABLE IF NOT EXISTS `sensor_reads`
(
    id           INTEGER  NOT NULL PRIMARY KEY AUTOINCREMENT,
    sensor_id    INTEGER  NOT NULL,
    sensor_value TEXT     NOT NULL,
    created_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   DATETIME NULL,
    FOREIGN KEY (sensor_id) REFERENCES sensors (id)
);

CREATE INDEX sensor_reads_sensor_id_index ON sensor_reads (sensor_id);