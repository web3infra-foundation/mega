CREATE TABLE IF NOT EXISTS `config_entry` (
    `id` INTEGER PRIMARY KEY AUTOINCREMENT,
    `configuration` TEXT NOT NULL,
    `name` TEXT,
    `key` TEXT NOT NULL,
    `value` TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS `reference` (
    `id` INTEGER PRIMARY KEY AUTOINCREMENT,
    `name` TEXT NOT NULL,
    `kind` TEXT NOT NULL CHECK (kind IN ('Branch', 'Tag', 'Head')),
    `commit` TEXT,
    -- remote can't be ''. If kind is Tag, remote must be NULL.
    `remote` TEXT CHECK (remote <> '' OR remote IS NULL),
    CHECK (
        (kind <> 'Tag' OR (kind = 'Tag' AND remote IS NULL))
    )
);