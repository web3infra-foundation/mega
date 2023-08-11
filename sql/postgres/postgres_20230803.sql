
CREATE TABLE IF NOT EXISTS "commit" (
  "id" SERIAL PRIMARY KEY,
  "git_id" VARCHAR(40) NOT NULL,
  "tree" VARCHAR(40) NOT NULL,
  "pid" VARCHAR(40),
  "meta" BYTEA NOT NULL,
  "repo_path" VARCHAR(128) NOT NULL,
  "author" VARCHAR(64),
  "committer" VARCHAR(64),
  "content" VARCHAR(128),
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL
);

CREATE TABLE IF NOT EXISTS "git" (
  "id" BIGINT NOT NULL,
  "mr_id" BIGINT NOT NULL,
  "git_id" VARCHAR(64),
  "object_type" VARCHAR(16),
  "created_at" TIMESTAMP NOT NULL,
  PRIMARY KEY ("id")
);

CREATE INDEX "idx_hash" ON "git" ("git_id");
CREATE INDEX "idx_mr_id" ON "git" ("mr_id", "object_type");


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

CREATE TABLE IF NOT EXISTS "node" (
  "id" BIGSERIAL PRIMARY KEY,
  "node_id" BIGINT NOT NULL,
  "git_id" VARCHAR(64) NOT NULL,
  "node_type" VARCHAR(16) NOT NULL,
  "name" VARCHAR(128),
  "mode" BYTEA NOT NULL,
  "content_sha" VARCHAR(40),
  "size" INT NOT NULL,
  "repo_path" VARCHAR(64) NOT NULL,
  "full_path" VARCHAR(64) NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL
);

CREATE INDEX "idx_git_id" ON "node" ("git_id");
CREATE INDEX "idx_name" ON "node" ("name");
CREATE INDEX "idx_repo_path" ON "node" ("repo_path");

CREATE TABLE IF NOT EXISTS "refs" (
  "id" SERIAL PRIMARY KEY,
  "repo_path" VARCHAR(64) NOT NULL,
  "ref_name" VARCHAR(32) NOT NULL,
  "ref_git_id" VARCHAR(40) NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL
);


CREATE TABLE IF NOT EXISTS "obj_data" (
  "id" BIGINT NOT NULL,
  "git_id" VARCHAR(64),
  "object_type" VARCHAR(16),
  "data" BYTEA,
  PRIMARY KEY ("id")
);
CREATE INDEX "idx_data_git_id" ON "obj_data" ("git_id");