CREATE TABLE IF NOT EXISTS `sensor_reads`
(
    id           int(11)      NOT NULL PRIMARY KEY AUTO_INCREMENT,
    sensor_id    int(11)      NOT NULL REFERENCES sensors (id),
    sensor_value varchar(255) NOT NULL,
    created_at   timestamp    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   timestamp    NULL     DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE = InnoDB
  DEFAULT CHARSET = utf8
  AUTO_INCREMENT = 1