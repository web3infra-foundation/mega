
CREATE TABLE IF NOT EXISTS "commit" (
  "id" SERIAL PRIMARY KEY,
  "git_id" VARCHAR(40) NOT NULL,
  "tree" VARCHAR(40) NOT NULL,
  "pid" TEXT[],
  "meta" BYTEA NOT NULL,
  "repo_path" VARCHAR(128) NOT NULL,
  "author" TEXT,
  "committer" TEXT,
  "content" TEXT,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL
);
CREATE INDEX "idx_c_git_id" ON "commit" ("git_id");
CREATE INDEX "idx_c_tree" ON "commit" ("tree");
CREATE INDEX "idx_c_repo_path" ON "commit" ("repo_path");


CREATE TABLE IF NOT EXISTS "node" (
  "id" BIGSERIAL PRIMARY KEY,
  "node_id" BIGINT NOT NULL,
  "git_id" VARCHAR(64) NOT NULL,
  "node_type" VARCHAR(16) NOT NULL,
  "name" VARCHAR(128),
  "mode" BYTEA NOT NULL,
  "content_sha" VARCHAR(40),
  "size" INT NOT NULL,
  "repo_path" VARCHAR(256) NOT NULL,
  "full_path" VARCHAR(512) NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL
);
CREATE INDEX "idx_node_git_id" ON "node" ("git_id");
CREATE INDEX "idx_node_name" ON "node" ("name");
CREATE INDEX "idx_node_repo_path" ON "node" ("repo_path");


CREATE TABLE IF NOT EXISTS "refs" (
  "id" SERIAL PRIMARY KEY,
  "repo_path" VARCHAR(64) NOT NULL,
  "ref_name" VARCHAR(32) NOT NULL,
  "ref_git_id" VARCHAR(40) NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL
);



CREATE TABLE IF NOT EXISTS "mr" (
  "id" BIGINT NOT NULL,
  "mr_id" BIGINT NOT NULL,
  "git_id" VARCHAR(40),
  "object_type" VARCHAR(16),
  "created_at" TIMESTAMP NOT NULL,
  PRIMARY KEY ("id")
);
CREATE INDEX "idx_mr_hash" ON "mr" ("git_id");
CREATE INDEX "idx_mr_id" ON "mr" ("mr_id", "object_type");


CREATE TABLE IF NOT EXISTS "git_obj" (
  "id" BIGINT NOT NULL,
  "git_id" VARCHAR(40),
  "object_type" VARCHAR(16),
  "data" BYTEA,
  PRIMARY KEY ("id")
);
CREATE INDEX "idx_data_git_id" ON "git_obj" ("git_id");


CREATE TABLE IF NOT EXISTS "mr_info" (
  "id" SERIAL PRIMARY KEY,
  "mr_id" BIGINT NOT NULL,
  "mr_msg" VARCHAR(255) NOT NULL,
  "mr_date" TIMESTAMP NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL
);
CREATE INDEX "idx_info_mr_id" ON "mr_info" ("mr_id");



-- used for lfs feature
CREATE TABLE IF NOT EXISTS "locks" (
  "id" VARCHAR(200) NOT NULL,
  "data" VARCHAR(10000),
  PRIMARY KEY ("id")
);

CREATE TABLE IF NOT EXISTS "meta" (
  "oid" VARCHAR(100) NOT NULL,
  "size" INT,
  "exist" SMALLINT,
  PRIMARY KEY ("oid")
);