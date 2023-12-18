CREATE TABLE IF NOT EXISTS `sensors`
(
    id          int(11)      NOT NULL PRIMARY KEY AUTO_INCREMENT,
    name        varchar(255) NULL     DEFAULT 'Default',
    sensor_type varchar(255) NOT NULL,
    ip_address  varchar(255) NOT NULL,
    created_at  timestamp    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  timestamp    NULL     DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE = InnoDB
  DEFAULT CHARSET = utf8
  AUTO_INCREMENT = 1;

CREATE INDEX `sensors_sensor_type_index` ON `sensors` (`sensor_type` ASC);