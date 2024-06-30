CREATE TABLE IF NOT EXISTS "mega_commit" (
  "id" BIGINT PRIMARY KEY,
  "commit_id" VARCHAR(40) NOT NULL,
  "tree" VARCHAR(40) NOT NULL,
  "parents_id" TEXT [] NOT NULL,
  "author" TEXT,
  "committer" TEXT,
  "content" TEXT,
  "created_at" TIMESTAMP NOT NULL,
  CONSTRAINT uniq_mc_git_id UNIQUE (commit_id)
);
CREATE INDEX "idx_mc_git_id" ON "mega_commit" ("commit_id");
CREATE TABLE IF NOT EXISTS "mega_tree" (
  "id" BIGINT PRIMARY KEY,
  "tree_id" VARCHAR(40) NOT NULL,
  "sub_trees" BYTEA NOT NULL,
  "size" INT NOT NULL,
  "commit_id" VARCHAR(40) NOT NULL,
  "created_at" TIMESTAMP NOT NULL
);
CREATE INDEX "idx_mt_git_id" ON "mega_tree" ("tree_id");
CREATE TABLE IF NOT EXISTS "mega_blob" (
  "id" BIGINT PRIMARY KEY,
  "blob_id" VARCHAR(40) NOT NULL,
  "commit_id" VARCHAR(40) NOT NULL,
  "name" TEXT NOT NULL,
  "size" INT NOT NULL,
  "created_at" TIMESTAMP NOT NULL
);
CREATE INDEX "idx_mb_git_id" ON "mega_blob" ("blob_id");
CREATE TABLE IF NOT EXISTS "mega_tag" (
  "id" BIGINT PRIMARY KEY,
  "tag_id" VARCHAR(40) NOT NULL,
  "object_id" VARCHAR(40) NOT NULL,
  "object_type" VARCHAR(20) NOT NULL,
  "tag_name" TEXT NOT NULL,
  "tagger" TEXT NOT NULL,
  "message" TEXT NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  CONSTRAINT uniq_mtag_tag_id UNIQUE (tag_id)
);
CREATE TABLE IF NOT EXISTS "mega_mr" (
  "id" BIGINT PRIMARY KEY,
  "mr_link" VARCHAR(40) NOT NULL,
  "merge_date" TIMESTAMP,
  "status" VARCHAR(20) NOT NULL,
  "path" TEXT NOT NULL,
  "from_hash" VARCHAR(40) NOT NULL,
  "to_hash" VARCHAR(40) NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL
);
CREATE INDEX "idx_mr_path" ON "mega_mr" ("path");

CREATE TABLE IF NOT EXISTS "mega_mr_conv" (
  "id" BIGINT PRIMARY KEY,
  "mr_id" BIGINT NOT NULL,
  "user_id" BIGINT NOT NULL,
  "conv_type"  VARCHAR(20) NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL
);
CREATE INDEX "idx_conversation" ON "mega_mr_conv" ("mr_id");


CREATE TABLE IF NOT EXISTS "mega_mr_comment" (
  "id" BIGINT PRIMARY KEY,
  "conv_id" BIGINT NOT NULL,
  "comment" TEXT,
  "edited" BOOLEAN NOT NULL
);
CREATE INDEX "idx_comment_id" ON "mega_mr_comment" ("conv_id");

CREATE TABLE IF NOT EXISTS "mega_issue" (
  "id" BIGINT PRIMARY KEY,
  "number" BIGINT NOT NULL,
  "title" VARCHAR(255) NOT NULL,
  "sender_name" VARCHAR(255) NOT NULL,
  "sender_id" BIGINT NOT NULL,
  "state" VARCHAR(255) NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL,
  "closed_at" TIMESTAMP DEFAULT NULL
);
CREATE TABLE IF NOT EXISTS "mega_refs" (
  "id" BIGINT PRIMARY KEY,
  "path" TEXT NOT NULL,
  "ref_commit_hash" VARCHAR(40) NOT NULL,
  "ref_tree_hash" VARCHAR(40) NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL,
  CONSTRAINT uniq_mref_path UNIQUE (path)
);
CREATE TABLE IF NOT EXISTS "import_refs" (
  "id" BIGINT PRIMARY KEY,
  "repo_id" BIGINT NOT NULL,
  "ref_name" TEXT NOT NULL,
  "ref_git_id" VARCHAR(40) NOT NULL,
  "ref_type" VARCHAR(20) NOT NULL,
  "default_branch" BOOLEAN NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL,
  CONSTRAINT uniq_ref_path_name UNIQUE (repo_id, ref_name)
);
CREATE INDEX "idx_refs_repo_id" ON "import_refs" ("repo_id");
CREATE TABLE IF NOT EXISTS "git_repo" (
  "id" BIGINT PRIMARY KEY,
  "repo_path" TEXT NOT NULL,
  "repo_name" TEXT NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL,
  CONSTRAINT uniq_ir_path UNIQUE (repo_path)
);
CREATE INDEX "idx_ir_repo_path" ON "git_repo" ("repo_path");
CREATE TABLE IF NOT EXISTS "git_commit" (
  "id" BIGINT PRIMARY KEY,
  "repo_id" BIGINT NOT NULL,
  "commit_id" VARCHAR(40) NOT NULL,
  "tree" VARCHAR(40) NOT NULL,
  "parents_id" TEXT [] NOT NULL,
  "author" TEXT,
  "committer" TEXT,
  "content" TEXT,
  "created_at" TIMESTAMP NOT NULL,
  CONSTRAINT uniq_c_git_repo_id UNIQUE (repo_id, commit_id)
);
CREATE INDEX "idx_ic_git_id" ON "git_commit" ("commit_id");
CREATE INDEX "idx_ic_repo_id" ON "git_commit" ("repo_id");
CREATE TABLE IF NOT EXISTS "git_tree" (
  "id" BIGINT PRIMARY KEY,
  "repo_id" BIGINT NOT NULL,
  "tree_id" VARCHAR(40) NOT NULL,
  "sub_trees" BYTEA NOT NULL,
  "size" INT NOT NULL,
  "commit_id" VARCHAR(40) NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  CONSTRAINT uniq_t_git_repo UNIQUE (repo_id, tree_id)
);
CREATE INDEX "idx_t_git_id" ON "git_tree" ("tree_id");
CREATE INDEX "idx_t_repo_id" ON "git_tree" ("repo_id");
CREATE TABLE IF NOT EXISTS "git_blob" (
  "id" BIGINT PRIMARY KEY,
  "repo_id" BIGINT NOT NULL,
  "blob_id" VARCHAR(40) NOT NULL,
  "name" VARCHAR(128),
  "size" INT NOT NULL,
  "commit_id" VARCHAR(40) NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  CONSTRAINT uniq_b_git_repo UNIQUE (repo_id, blob_id)
);
CREATE INDEX "idx_b_git_id" ON "git_blob" ("blob_id");
CREATE TABLE IF NOT EXISTS "git_tag" (
  "id" BIGINT PRIMARY KEY,
  "repo_id" BIGINT NOT NULL,
  "tag_id" VARCHAR(40) NOT NULL,
  "object_id" VARCHAR(40) NOT NULL,
  "object_type" VARCHAR(20) NOT NULL,
  "tag_name" TEXT NOT NULL,
  "tagger" TEXT NOT NULL,
  "message" TEXT NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  CONSTRAINT uniq_gtag_tag_id UNIQUE (tag_id)
);
CREATE TABLE IF NOT EXISTS "raw_blob" (
  "id" BIGINT PRIMARY KEY,
  "sha1" VARCHAR(40) NOT NULL,
  "content" TEXT,
  "file_type" VARCHAR(20),
  "storage_type" VARCHAR(20) NOT NULL,
  "data" BYTEA,
  "local_path" TEXT,
  "remote_url" TEXT,
  "created_at" TIMESTAMP NOT NULL,
  CONSTRAINT uniq_rb_sha1 UNIQUE (sha1)
);
CREATE INDEX "idx_rb_sha1" ON "raw_blob" ("sha1");
CREATE TABLE IF NOT EXISTS "git_pr" (
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
CREATE TABLE IF NOT EXISTS "git_issue" (
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
CREATE TABLE IF NOT EXISTS "lfs_locks" (
  "id" VARCHAR(40) PRIMARY KEY,
  "data" TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS "lfs_objects" (
  "oid" VARCHAR(64) PRIMARY KEY,
  "size" BIGINT NOT NULL,
  "exist" BOOLEAN NOT NULL,
  "splited" BOOLEAN NOT NULL,
);
CREATE TABLE IF NOT EXISTS "lfs_split_relations" (
    "ori_oid" VARCHAR(64) NOT NULL,
    "sub_oid" VARCHAR(64) NOT NULL,
    "offset" BIGINT NOT NULL,
    "size" BIGINT NOT NULL,
    PRIMARY KEY ("ori_oid", "sub_oid", "offset")
)