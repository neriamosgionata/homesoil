CREATE TABLE IF NOT EXISTS `scripts`
(
    id          INTEGER  NOT NULL PRIMARY KEY AUTOINCREMENT,
    title       TEXT     NOT NULL DEFAULT 'Default',
    code        TEXT     NOT NULL,
    schedule    TEXT     NULL,
    status      INTEGER  NOT NULL DEFAULT 0,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME NULL
);
