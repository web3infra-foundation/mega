CREATE TABLE IF NOT EXISTS "mega_directory"(
  "id" BIGINT PRIMARY KEY,
  "path" TEXT NOT NULL,
  "import_dir" BOOLEAN,
  "tree_id" VARCHAR(40),
  "sub_trees" TEXT [],
  "commit_id" VARCHAR(40),
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL,
  CONSTRAINT uniq_md_full_path UNIQUE (path)
);
CREATE TABLE IF NOT EXISTS "mega_commit" (
  "id" BIGINT PRIMARY KEY,
  "git_id" VARCHAR(40) NOT NULL,
  "tree" VARCHAR(40) NOT NULL,
  "pid" TEXT [],
  "author" TEXT,
  "committer" TEXT,
  "content" TEXT,
  "mr_id" VARCHAR(20),
  "status" VARCHAR(20) NOT NULL,
  "size" INT NOT NULL,
  "full_path" TEXT NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL,
  CONSTRAINT uniq_mc_git_id UNIQUE (git_id)
);
CREATE INDEX "idx_mc_git_id" ON "mega_commit" ("git_id");
CREATE TABLE IF NOT EXISTS "mega_tree" (
  "id" BIGINT PRIMARY KEY,
  "git_id" VARCHAR(40) NOT NULL,
  "sub_trees" TEXT [],
  "import_dir" BOOLEAN,
  "mr_id" VARCHAR(20),
  "status" VARCHAR(20) NOT NULL,
  "size" INT NOT NULL,
  "full_path" TEXT NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL,
  CONSTRAINT uniq_mt_git_id UNIQUE (git_id)
);
CREATE INDEX "idx_mt_git_id" ON "mega_tree" ("git_id");
CREATE TABLE IF NOT EXISTS "mega_blob" (
  "id" BIGINT PRIMARY KEY,
  "git_id" VARCHAR(40) NOT NULL,
  "commit_id" VARCHAR(40) NOT NULL,
  "mr_id" VARCHAR(20),
  "status" VARCHAR(20) NOT NULL,
  "size" INT NOT NULL,
  "full_path" TEXT NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL,
  CONSTRAINT uniq_mb_git_id UNIQUE (git_id)
);
CREATE INDEX "idx_mb_git_id" ON "mega_blob" ("git_id");
CREATE TABLE IF NOT EXISTS "merge_request" (
  "id" BIGINT PRIMARY KEY,
  "mr_id" BIGINT NOT NULL,
  "mr_msg" VARCHAR(255) NOT NULL,
  "commit_id" VARCHAR(40) NOT NULL,
  "mr_date" TIMESTAMP NOT NULL,
  "status" VARCHAR(20) NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL
);
CREATE INDEX "idx_info_mr_id" ON "merge_request" ("mr_id");
CREATE TABLE IF NOT EXISTS "import_refs" (
  "id" BIGINT PRIMARY KEY,
  "repo_id" BIGINT NOT NULL,
  "ref_name" TEXT NOT NULL,
  "ref_git_id" VARCHAR(40) NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL,
  CONSTRAINT uniq_ref_path_name UNIQUE (repo_id, ref_name)
);
CREATE INDEX "idx_refs_repo_id" ON "import_refs" ("repo_id");
CREATE TABLE IF NOT EXISTS "import_repo" (
  "id" BIGINT PRIMARY KEY,
  "repo_path" TEXT NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL,
  CONSTRAINT uniq_ir_path UNIQUE (repo_path)
);
CREATE INDEX "idx_ir_repo_path" ON "import_repo" ("repo_path");
CREATE TABLE IF NOT EXISTS "import_commit" (
  "id" BIGINT PRIMARY KEY,
  "git_id" VARCHAR(40) NOT NULL,
  "tree" VARCHAR(40) NOT NULL,
  "pid" TEXT [],
  "repo_id" BIGINT NOT NULL,
  "author" TEXT,
  "committer" TEXT,
  "content" TEXT,
  "size" INT NOT NULL,
  "full_path" TEXT NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  CONSTRAINT uniq_c_git_repo_id UNIQUE (repo_id, git_id)
);
CREATE INDEX "idx_ic_git_id" ON "import_commit" ("git_id");
CREATE INDEX "idx_ic_repo_id" ON "import_commit" ("repo_id");
CREATE TABLE IF NOT EXISTS "import_tree" (
  "id" BIGINT PRIMARY KEY,
  "repo_id" BIGINT NOT NULL,
  "git_id" VARCHAR(40) NOT NULL,
  "sub_trees" TEXT [],
  "name" VARCHAR(128),
  "size" INT NOT NULL,
  "full_path" TEXT NOT NULL,
  "commit_id" VARCHAR(40) NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  CONSTRAINT uniq_t_git_repo UNIQUE (repo_id, git_id)
);
CREATE INDEX "idx_t_git_id" ON "import_tree" ("git_id");
CREATE INDEX "idx_t_repo_id" ON "import_tree" ("repo_id");
CREATE TABLE IF NOT EXISTS "import_blob" (
  "id" BIGINT PRIMARY KEY,
  "repo_id" BIGINT NOT NULL,
  "git_id" VARCHAR(40) NOT NULL,
  "name" VARCHAR(128),
  "size" INT NOT NULL,
  "full_path" TEXT NOT NULL,
  "commit_id" VARCHAR(40) NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  CONSTRAINT uniq_b_git_repo UNIQUE (repo_id, git_id)
);
CREATE INDEX "idx_b_git_id" ON "import_blob" ("git_id");
CREATE TABLE IF NOT EXISTS "raw_objects" (
  "id" BIGINT PRIMARY KEY,
  "git_id" VARCHAR(40) NOT NULL,
  "object_type" VARCHAR(20) NOT NULL,
  "storage_type" VARCHAR(20) NOT NULL,
  "data" BYTEA,
  "local_storage_path" TEXT,
  "remote_url" TEXT,
  CONSTRAINT uniq_ro_git_id UNIQUE (git_id)
);
CREATE INDEX "idx_ro_git_id" ON "raw_objects" ("git_id");
CREATE TABLE IF NOT EXISTS "pull_request" (
  "id" BIGINT PRIMARY KEY,
  "number" BIGINT NOT NULL,
  "title" VARCHAR(255) NOT NULL,
  "state" VARCHAR(255) NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL,
  "closed_at" TIMESTAMP DEFAULT NULL,
  "merged_at" TIMESTAMP DEFAULT NULL,
  "merge_commit_sha" VARCHAR(200) DEFAULT NULL,
  "repo_id" BIGINT NOT NULL,
  "sender_name" VARCHAR(255) NOT NULL,
  "sender_id" BIGINT NOT NULL,
  "user_name" VARCHAR(255) NOT NULL,
  "user_id" BIGINT NOT NULL,
  "commits_url" VARCHAR(255) NOT NULL,
  "patch_url" VARCHAR(255) NOT NULL,
  "head_label" VARCHAR(255) NOT NULL,
  "head_ref" VARCHAR(255) NOT NULL,
  "base_label" VARCHAR(255) NOT NULL,
  "base_ref" VARCHAR(255) NOT NULL
);
CREATE TABLE IF NOT EXISTS "issue" (
  "id" BIGINT PRIMARY KEY,
  "number" BIGINT NOT NULL,
  "title" VARCHAR(255) NOT NULL,
  "sender_name" VARCHAR(255) NOT NULL,
  "sender_id" BIGINT NOT NULL,
  "state" VARCHAR(255) NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL,
  "closed_at" TIMESTAMP DEFAULT NULL,
  "repo_id" BIGINT NOT NULL
);
CREATE TABLE IF NOT EXISTS "lfs_locks" ("id" VARCHAR(40) PRIMARY KEY, "data" TEXT);
CREATE TABLE IF NOT EXISTS "lfs_objects" (
  "oid" VARCHAR(64) PRIMARY KEY,
  "size" BIGINT,
  "repo_id" BIGINT NOT NULL,
  "exist" BOOLEAN
);