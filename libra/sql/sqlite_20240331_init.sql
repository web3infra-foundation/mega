CREATE TABLE IF NOT EXISTS `config` (
    `id` INTEGER PRIMARY KEY AUTOINCREMENT,
    `configuration` TEXT NOT NULL,
    `name` TEXT,
    `key` TEXT NOT NULL,
    `value` TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS `reference` (
    `id` INTEGER PRIMARY KEY AUTOINCREMENT,
    -- name can't be ''
    `name` TEXT CHECK (name <> '' OR name IS NULL),
    `kind` TEXT NOT NULL CHECK (kind IN ('Branch', 'Tag', 'Head')),
    `commit` TEXT,
    -- remote can't be ''. If kind is Tag, remote must be NULL.
    `remote` TEXT CHECK (remote <> '' OR remote IS NULL),
    CHECK (
        (kind <> 'Tag' OR (kind = 'Tag' AND remote IS NULL))
    )
);
CREATE TABLE IF NOT EXISTS `reflog` (
    `id`              INTEGER PRIMARY KEY AUTOINCREMENT,
    `ref_name`        TEXT NOT NULL,
    `old_oid`         TEXT NOT NULL,
    `new_oid`         TEXT NOT NULL,
    `committer_name`  TEXT NOT NULL,
    `committer_email` TEXT NOT NULL,
    `timestamp`       INTEGER NOT NULL,
    `action`          TEXT NOT NULL,
    `message`         TEXT NOT NULL
);
--  (name, kind, remote) as unique key when remote is not null
CREATE UNIQUE INDEX idx_name_kind_remote ON `reference`(`name`, `kind`, `remote`)
WHERE `remote` IS NOT NULL;

-- (name, kind) as unique key when remote is null
CREATE UNIQUE INDEX idx_name_kind ON `reference`(`name`, `kind`)
WHERE `remote` IS NULL;

CREATE INDEX idx_ref_name_timestamp ON `reflog`(`ref_name`, `timestamp`)