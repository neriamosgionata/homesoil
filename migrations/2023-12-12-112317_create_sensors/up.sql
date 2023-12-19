CREATE TABLE IF NOT EXISTS `sensors`
(
    id          INTEGER  NOT NULL PRIMARY KEY AUTOINCREMENT,
    name        TEXT     NULL     DEFAULT 'Default',
    sensor_type TEXT     NOT NULL,
    ip_address  TEXT     NOT NULL,
    port        SMALLINT NOT NULL,
    online      TINYINT  NOT NULL DEFAULT 0,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME NULL
);

CREATE INDEX `sensors_sensor_type_index` ON `sensors` (`sensor_type` ASC);