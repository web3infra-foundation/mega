
CREATE TABLE IF NOT EXISTS `commit` (
  `id` int NOT NULL AUTO_INCREMENT,
  `git_id` varchar(40) NOT NULL,
  `tree` varchar(40) NOT NULL,
  -- fix array arguments later 
  `pid` varchar(40) DEFAULT NULL,
  `meta` blob NOT NULL,
  `repo_path` varchar(128) NOT NULL,
  `author` TINYTEXT DEFAULT NULL,
  `committer` TINYTEXT DEFAULT NULL,
  `content` TINYTEXT DEFAULT NULL,
  `created_at` datetime NOT NULL,
  `updated_at` datetime NOT NULL,
  PRIMARY KEY (`id`),
  KEY `idx_git_id` (`git_id`),
  KEY `idx_tree` (`tree`),
  KEY `idx_repo_path` (`repo_path`)
);


CREATE TABLE IF NOT EXISTS `node` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `node_id` bigint NOT NULL,
  `git_id` varchar(64) NOT NULL,
  `node_type` varchar(16) NOT NULL,
  `name` varchar(128) DEFAULT NULL,
  `mode` blob NOT NULL,
  `content_sha` varchar(40) DEFAULT NULL,
  `size` INT NOT NULL,
  `data` mediumblob NOT NULL,
  `repo_path` VARCHAR(256) NOT NULL,
  `full_path` VARCHAR(512) NOT NULL,
  `created_at` datetime NOT NULL,
  `updated_at` datetime NOT NULL,
  PRIMARY KEY (`id`),
  KEY `idx_git_id` (`git_id`),
  KEY `idx_name` (`name`),
  KEY `idx_repo_path` (`repo_path`)
);


CREATE TABLE IF NOT EXISTS `refs` (
  `id` int NOT NULL AUTO_INCREMENT,
  `repo_path` varchar(64) NOT NULL,
  `ref_name` varchar(32) NOT NULL,
  `ref_git_id` varchar(40) NOT NULL,
  `created_at` datetime NOT NULL,
  `updated_at` datetime NOT NULL,
  PRIMARY KEY (`id`)
);


CREATE TABLE IF NOT EXISTS `mr` (
  `id` BIGINT NOT NULL,
  `mr_id` BIGINT NOT NULL,
  `git_id` varchar(40) NOT NULL,
  `object_type` varchar(16) NOT NULL,
  `created_at` datetime NOT NULL,
  PRIMARY KEY (`id`),
  KEY `idx_hash` (`git_id`),
  KEY `idx_mr_id` (`mr_id`,`object_type`)
);


CREATE TABLE IF NOT EXISTS git_obj (
  `id` BIGINT NOT NULL,
  `git_id` VARCHAR(40),
  `object_type` VARCHAR(16),
  `data` mediumblob NOT NULL,
  PRIMARY KEY (`id`),
  KEY `idx_data_git_id` (`git_id`)
);

CREATE TABLE IF NOT EXISTS mr_info (
  `id` BIGINT NOT NULL AUTO_INCREMENT,
  `mr_id` BIGINT NOT NULL,
  `mr_msg` VARCHAR(255) NOT NULL,
  `mr_date` datetime NOT NULL,
  `created_at` datetime NOT NULL,
  `updated_at` datetime NOT NULL,
  PRIMARY KEY (`id`),
  KEY `idx_mr_id` (`mr_id`)
);



-- used for lfs feature
CREATE TABLE IF NOT EXISTS `locks` (
  `id` varchar(200) NOT NULL,
  `data` varchar(10000) DEFAULT NULL,
  PRIMARY KEY (`id`)
);

CREATE TABLE IF NOT EXISTS `meta` (
  `oid` varchar(100) NOT NULL,
  `size` int DEFAULT NULL,
  `exist` tinyint DEFAULT NULL,
  PRIMARY KEY (`oid`)
);
