CREATE TABLE IF NOT EXISTS `sensors`
(
    id          int(11)      NOT NULL PRIMARY KEY AUTO_INCREMENT,
    sensor_type varchar(255) NOT NULL,
    ip_address  varchar(255) NOT NULL,
    name        varchar(255) NOT NULL
) ENGINE = InnoDB
  DEFAULT CHARSET = utf8
  AUTO_INCREMENT = 1