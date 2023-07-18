CREATE TABLE `git` (
  `id` bigint NOT NULL,
  `mr_id` bigint NOT NULL,
  `git_id` varchar(64) NOT NULL,
  `object_type` varchar(16) NOT NULL,
  `data` blob NOT NULL,
  `created_at` datetime NOT NULL,
  `updated_at` datetime NOT NULL,
  PRIMARY KEY (`id`),
  KEY `idx_mr_id` (`mr_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci