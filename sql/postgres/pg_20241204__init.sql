CREATE TABLE IF NOT EXISTS "mega_commit" (
  "id" BIGINT PRIMARY KEY,
  "commit_id" VARCHAR(40) NOT NULL,
  "tree" VARCHAR(40) NOT NULL,
  "parents_id" JSON NOT NULL,  -- for compatibility with sqlite, DO NOT use Array Type
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
  "link" VARCHAR(40) NOT NULL,
  "title" TEXT NOT NULL,
  "merge_date" TIMESTAMP,
  "status" VARCHAR(20) NOT NULL,
  "path" TEXT NOT NULL,
  "from_hash" VARCHAR(40) NOT NULL,
  "to_hash" VARCHAR(40) NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL
);
CREATE INDEX "idx_mr_path" ON "mega_mr" ("path");

CREATE TABLE IF NOT EXISTS "mega_conversation" (
  "id" BIGINT PRIMARY KEY,
  "link" VARCHAR(20) NOT NULL,
  "user_id" BIGINT NOT NULL,
  "conv_type"  VARCHAR(20) NOT NULL,
  "comment" TEXT,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL
);
CREATE INDEX "idx_conversation" ON "mega_conversation" ("link");


CREATE TABLE IF NOT EXISTS "mega_issue" (
  "id" BIGINT PRIMARY KEY,
  "link"  VARCHAR(20) NOT NULL,
  "title" VARCHAR(255) NOT NULL,
  "owner" BIGINT NOT NULL,
  "status" VARCHAR(20) NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL,
  "closed_at" TIMESTAMP DEFAULT NULL
);
CREATE INDEX "idx_issue" ON "mega_issue" ("link");

CREATE TABLE IF NOT EXISTS "mega_refs" (
  "id" BIGINT PRIMARY KEY,
  "path" TEXT NOT NULL,
  "ref_name" TEXT NOT NULL,
  "ref_commit_hash" VARCHAR(40) NOT NULL,
  "ref_tree_hash" VARCHAR(40) NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL,
  CONSTRAINT uniq_mref_path UNIQUE (path, ref_name)
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
  "parents_id" JSON NOT NULL,
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
  "splited" BOOLEAN NOT NULL
);
CREATE TABLE IF NOT EXISTS "lfs_split_relations" (
    "ori_oid" VARCHAR(64) NOT NULL,
    "sub_oid" VARCHAR(64) NOT NULL,
    "offset" BIGINT NOT NULL,
    "size" BIGINT NOT NULL,
    PRIMARY KEY ("ori_oid", "sub_oid", "offset")
);

CREATE TABLE IF NOT EXISTS "relay_node" (
  "peer_id" VARCHAR(64) PRIMARY KEY,
  "type" VARCHAR(64),
  "online" BOOLEAN NOT NULL,
  "last_online_time" BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS "relay_repo_info" (
  "identifier" VARCHAR(128) PRIMARY KEY,
  "name" VARCHAR(64),
  "origin" VARCHAR(64),
  "update_time" BIGINT NOT NULL,
  "commit" VARCHAR(64)
);

CREATE TABLE IF NOT EXISTS "relay_lfs_info" (
  "id" BIGINT PRIMARY KEY,
  "file_hash" VARCHAR(256),
  "hash_type" VARCHAR(64),
  "file_size" BIGINT NOT NULL,
  "creation_time" BIGINT NOT NULL,
  "peer_id" VARCHAR(64),
  "origin" VARCHAR(256)
);

CREATE TABLE IF NOT EXISTS "relay_nostr_event" (
  "id" VARCHAR(128) PRIMARY KEY,
  "pubkey" VARCHAR(128),
  "created_at" BIGINT NOT NULL,
  "kind" INT,
  "tags" TEXT,
  "content" TEXT,
  "sig" VARCHAR(256)
);

CREATE TABLE IF NOT EXISTS "relay_nostr_req" (
  "id" VARCHAR(128) PRIMARY KEY,
  "subscription_id" VARCHAR(128),
  "filters" TEXT
);

CREATE TABLE IF NOT EXISTS "mq_storage" (
  "id" BIGINT PRIMARY KEY,
  "category" VARCHAR(64),
  "create_time" TIMESTAMP NOT NULL,
  "content" TEXT
);

CREATE TABLE IF NOT EXISTS "relay_path_mapping" (
  "id" BIGINT PRIMARY KEY,
  "alias" TEXT NOT NULL,
  "repo_path" TEXT NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP NOT NULL,
  CONSTRAINT uniq_alias UNIQUE (alias)
);

CREATE TABLE IF NOT EXISTS "user" (
  "id" BIGINT PRIMARY KEY,
  "name" TEXT NOT NULL,
  "email" TEXT NOT NULL,
  "avatar_url" TEXT NOT NULL,
  "is_github" BOOLEAN NOT NULL,
  "created_at" TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP,
  CONSTRAINT uniq_email UNIQUE (email)
);

CREATE TABLE IF NOT EXISTS "ssh_keys" (
  "id" BIGINT PRIMARY KEY,
  "user_id" BIGINT NOT NULL,
  "title" TEXT NOT NULL,
  "ssh_key" TEXT NOT NULL,
  "finger" TEXT NOT NULL,
  "created_at" TIMESTAMP NOT NULL
);
CREATE INDEX "idx_user_id" ON "ssh_keys" ("user_id");
CREATE INDEX "idx_ssh_key_finger" ON "ssh_keys" ((left(finger, 8)));


CREATE TABLE IF NOT EXISTS "access_token" (
  "id" BIGINT PRIMARY KEY,
  "user_id" BIGINT NOT NULL,
  "token" TEXT NOT NULL,
  "created_at" TIMESTAMP NOT NULL
);
CREATE INDEX "idx_token_user_id" ON "access_token" ("user_id");
CREATE INDEX "idx_token" ON "access_token" ((left(token, 8)));


CREATE TABLE IF NOT EXISTS "builds" (
  "build_id" uuid NOT NULL PRIMARY KEY,
  "output" varchar NOT NULL,
  "exit_code" integer,
  "start_at" timestamp with time zone NOT NULL,
  "end_at" timestamp with time zone NOT NULL,
  "repo_name" varchar NOT NULL,
  "target" varchar NOT NULL
);