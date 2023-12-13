CREATE TABLE IF NOT EXISTS `sensor_reads`
(
    id           int(11)      NOT NULL PRIMARY KEY AUTO_INCREMENT,
    sensor_id    int(11)      NOT NULL REFERENCES sensors (id),
    sensor_value varchar(255) NOT NULL
) ENGINE = InnoDB
  DEFAULT CHARSET = utf8
  AUTO_INCREMENT = 1