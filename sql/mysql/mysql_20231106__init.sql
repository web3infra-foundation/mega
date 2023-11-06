CREATE TABLE IF NOT EXISTS `commit` (
  `id` INT AUTO_INCREMENT PRIMARY KEY,
  `git_id` VARCHAR(40) NOT NULL,
  `tree` VARCHAR(40) NOT NULL,
  `pid` TEXT,
  `repo_path` VARCHAR(128) NOT NULL,
  `author` TEXT,
  `committer` TEXT,
  `content` TEXT,
  `created_at` TIMESTAMP NOT NULL,
  `updated_at` TIMESTAMP NOT NULL,
  UNIQUE KEY `uniq_c_git_id` (`git_id`),
  KEY `idx_git_id` (`git_id`),
  KEY `idx_tree` (`tree`),
  KEY `idx_repo_path` (`repo_path`)
  );

CREATE TABLE IF NOT EXISTS `node` (
  `id` BIGINT AUTO_INCREMENT PRIMARY KEY,
  `node_id` BIGINT NOT NULL,
  `git_id` VARCHAR(40) NOT NULL,
  `last_commit` VARCHAR(40) NOT NULL,
  `node_type` VARCHAR(16) NOT NULL,
  `name` VARCHAR(128),
  `mode` VARBINARY(255) NOT NULL,
  `content_sha` VARCHAR(40),
  `size` INT NOT NULL,
  `repo_path` VARCHAR(256) NOT NULL,
  `full_path` VARCHAR(512) NOT NULL,
  `created_at` TIMESTAMP NOT NULL,
  `updated_at` TIMESTAMP NOT NULL,
  UNIQUE KEY `uniq_n_git_id` (`git_id`),
  KEY `idx_git_id` (`git_id`),
  KEY `idx_name` (`name`),
  KEY `idx_repo_path` (`repo_path`)
);

CREATE TABLE IF NOT EXISTS `refs` (
  `id` INT AUTO_INCREMENT PRIMARY KEY,
  `repo_path` VARCHAR(64) NOT NULL,
  `ref_name` VARCHAR(64) NOT NULL,
  `ref_git_id` VARCHAR(40) NOT NULL,
  `created_at` TIMESTAMP NOT NULL,
  `updated_at` TIMESTAMP NOT NULL
);

CREATE TABLE IF NOT EXISTS `mr` (
  `id` BIGINT NOT NULL,
  `mr_id` BIGINT NOT NULL,
  `git_id` VARCHAR(40),
  `object_type` VARCHAR(16),
  `created_at` TIMESTAMP NOT NULL,
  PRIMARY KEY (`id`),
  KEY `idx_hash` (`git_id`),
  KEY `idx_mr_id` (`mr_id`,`object_type`)
);

CREATE TABLE IF NOT EXISTS `objects` (
  `id` BIGINT NOT NULL,
  `git_id` VARCHAR(40) NOT NULL,
  `object_type` VARCHAR(16) NOT NULL,
  `data` LONGBLOB,
  `link` VARCHAR(512),
  PRIMARY KEY (`id`),
  UNIQUE KEY `uniq_o_git_id` (`git_id`)
);

CREATE TABLE IF NOT EXISTS `mr_info` (
  `id` INT AUTO_INCREMENT PRIMARY KEY,
  `mr_id` BIGINT NOT NULL,
  `mr_msg` VARCHAR(255) NOT NULL,
  `mr_date` TIMESTAMP NOT NULL,
  `created_at` TIMESTAMP NOT NULL,
  `updated_at` TIMESTAMP NOT NULL,
  KEY `idx_mr_id` (`mr_id`)
);

CREATE TABLE IF NOT EXISTS `locks` (
  `id` VARCHAR(200) NOT NULL,
  `data` VARCHAR(10000),
  PRIMARY KEY (`id`)
);

CREATE TABLE IF NOT EXISTS `meta` (
  `oid` VARCHAR(64) NOT NULL,
  `size` BIGINT,
  `exist` TINYINT(1),
  PRIMARY KEY (`oid`)
);

CREATE TABLE IF NOT EXISTS `issue` (
  `id` BIGINT PRIMARY KEY,
  `number` BIGINT NOT NULL,
  `title` VARCHAR(255) NOT NULL,
  `sender_name` VARCHAR(255) NOT NULL,
  `sender_id` BIGINT NOT NULL,
  `state` VARCHAR(255) NOT NULL,
  `created_at` TIMESTAMP NOT NULL,
  `updated_at` TIMESTAMP NOT NULL,
  `closed_at` TIMESTAMP,
  `repo_path` VARCHAR(255) NOT NULL,
  `repo_id` BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS `repo_directory` (
  `id` INT AUTO_INCREMENT PRIMARY KEY,
  `pid` INT NOT NULL DEFAULT 0,
  `name` VARCHAR(255) NOT NULL,
  `is_repo` TINYINT(1) NOT NULL,
  `full_path` TEXT NOT NULL,
  `created_at` TIMESTAMP NOT NULL,
  `updated_at` TIMESTAMP NOT NULL,
  UNIQUE KEY `uniq_d_full_path` (`full_path`)
);

CREATE TABLE IF NOT EXISTS `pull_request` (
  `id` BIGINT PRIMARY KEY,
  `number` BIGINT NOT NULL,
  `title` VARCHAR(255) NOT NULL,
  `state` VARCHAR(255) NOT NULL,
  `created_at` TIMESTAMP NOT NULL,
  `updated_at` TIMESTAMP NOT NULL,
  `closed_at` TIMESTAMP,
  `merged_at` TIMESTAMP,
  `merge_commit_sha` VARCHAR(200),
  `repo_path` VARCHAR(255) NOT NULL,
  `repo_id` BIGINT NOT NULL,
  `sender_name` VARCHAR(255) NOT NULL,
  `sender_id` BIGINT NOT NULL,
  `user_name` VARCHAR(255) NOT NULL,
  `user_id` BIGINT NOT NULL,
  `commits_url` VARCHAR(255) NOT NULL,
  `patch_url` VARCHAR(255) NOT NULL,
  `head_label` VARCHAR(255) NOT NULL,
  `head_ref` VARCHAR(255) NOT NULL,
  `base_label` VARCHAR(255) NOT NULL,
  `base_ref` VARCHAR(255) NOT NULL
);
