CREATE TABLE IF NOT EXISTS `actuators`
(
    id         int(11)      NOT NULL PRIMARY KEY AUTO_INCREMENT,
    name       varchar(255) NULL     DEFAULT 'Default',
    ip_address varchar(255) NOT NULL,
    state      tinyint(1)   NOT NULL DEFAULT 0,
    online     tinyint(1)   NOT NULL DEFAULT 0,
    created_at timestamp    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp    NULL     DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE = InnoDB
  DEFAULT CHARSET = utf8
  AUTO_INCREMENT = 1;
