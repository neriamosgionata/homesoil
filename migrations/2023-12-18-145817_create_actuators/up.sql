CREATE TABLE IF NOT EXISTS `actuators`
(
    id         INTEGER  NOT NULL PRIMARY KEY AUTOINCREMENT,
    name       TEXT     NULL     DEFAULT 'Default',
    ip_address TEXT     NOT NULL,
    port       SMALLINT NOT NULL,
    state      TINYINT  NOT NULL DEFAULT 0,
    online     TINYINT  NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NULL
);
