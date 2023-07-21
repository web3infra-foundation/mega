ALTER TABLE `mega`.`node` 
ADD COLUMN `repo_path` VARCHAR(64) NOT NULL AFTER `data`,
ADD INDEX `idx_repo_path` (`repo_path`);


ALTER TABLE `mega`.`commit` 
ADD INDEX `idx_repo_path` (`repo_path`);


