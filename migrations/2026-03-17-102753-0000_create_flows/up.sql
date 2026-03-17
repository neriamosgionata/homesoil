CREATE TABLE IF NOT EXISTS `flows`
(
    id          INTEGER  NOT NULL PRIMARY KEY AUTOINCREMENT,
    title       TEXT     NOT NULL DEFAULT 'Default',
    graph       TEXT     NOT NULL DEFAULT '{}',
    enabled     TINYINT  NOT NULL DEFAULT 0,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME NULL
);
