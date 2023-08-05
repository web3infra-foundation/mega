
DROP TABLE IF EXISTS "commit";
CREATE TABLE "commit" (
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

DROP TABLE IF EXISTS "git";
CREATE TABLE "git" (
  "id" BIGINT NOT NULL,
  "mr_id" BIGINT NOT NULL,
  "git_id" VARCHAR(64),
  "object_type" VARCHAR(16),
  "data" BYTEA,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL,
  PRIMARY KEY ("id")
);

CREATE INDEX "idx_hash" ON "git" ("git_id");
CREATE INDEX "idx_mr_id" ON "git" ("mr_id", "object_type");

DROP TABLE IF EXISTS "locks";
CREATE TABLE "locks" (
  "id" VARCHAR(200) NOT NULL,
  "data" VARCHAR(10000),
  PRIMARY KEY ("id")
);

DROP TABLE IF EXISTS "meta";
CREATE TABLE "meta" (
  "oid" VARCHAR(100) NOT NULL,
  "size" INT,
  "exist" SMALLINT,
  PRIMARY KEY ("oid")
);

DROP TABLE IF EXISTS "node";
CREATE TABLE "node" (
  "id" BIGSERIAL PRIMARY KEY,
  "node_id" BIGINT NOT NULL,
  "git_id" VARCHAR(64) NOT NULL,
  "node_type" VARCHAR(16) NOT NULL,
  "name" VARCHAR(128),
  "mode" BYTEA NOT NULL,
  "content_sha" VARCHAR(40),
  "data" BYTEA NOT NULL,
  "repo_path" VARCHAR(64) NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL
);

CREATE INDEX "idx_git_id" ON "node" ("git_id");
CREATE INDEX "idx_name" ON "node" ("name");
CREATE INDEX "idx_repo_path" ON "node" ("repo_path");

DROP TABLE IF EXISTS "refs";
CREATE TABLE "refs" (
  "id" SERIAL PRIMARY KEY,
  "repo_path" VARCHAR(64) NOT NULL,
  "ref_name" VARCHAR(32) NOT NULL,
  "ref_git_id" VARCHAR(40) NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL
);
