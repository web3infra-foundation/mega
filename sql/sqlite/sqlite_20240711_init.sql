CREATE TABLE IF NOT EXISTS "mega_commit" (
  "id" INTEGER PRIMARY KEY,
  "commit_id" TEXT NOT NULL,
  "tree" TEXT NOT NULL,
  "parents_id" TEXT NOT NULL,  -- Use JSON to store array
  "author" TEXT,
  "committer" TEXT,
  "content" TEXT,
  "created_at" TEXT NOT NULL,
  CONSTRAINT uniq_mc_git_id UNIQUE (commit_id)
);
CREATE INDEX "idx_mc_git_id" ON "mega_commit" ("commit_id");
CREATE TABLE IF NOT EXISTS "mega_tree" (
  "id" INTEGER PRIMARY KEY,
  "tree_id" TEXT NOT NULL,
  "sub_trees" BLOB NOT NULL,
  "size" INTEGER NOT NULL,
  "commit_id" TEXT NOT NULL,
  "created_at" TEXT NOT NULL
);
CREATE INDEX "idx_mt_git_id" ON "mega_tree" ("tree_id");
CREATE TABLE IF NOT EXISTS "mega_blob" (
  "id" INTEGER PRIMARY KEY,
  "blob_id" TEXT NOT NULL,
  "commit_id" TEXT NOT NULL,
  "name" TEXT NOT NULL,
  "size" INTEGER NOT NULL,
  "created_at" TEXT NOT NULL
);
CREATE INDEX "idx_mb_git_id" ON "mega_blob" ("blob_id");
CREATE TABLE IF NOT EXISTS "mega_tag" (
  "id" INTEGER PRIMARY KEY,
  "tag_id" TEXT NOT NULL,
  "object_id" TEXT NOT NULL,
  "object_type" TEXT NOT NULL,
  "tag_name" TEXT NOT NULL,
  "tagger" TEXT NOT NULL,
  "message" TEXT NOT NULL,
  "created_at" TEXT NOT NULL,
  CONSTRAINT uniq_mtag_tag_id UNIQUE (tag_id)
);
CREATE TABLE IF NOT EXISTS "mega_mr" (
  "id" INTEGER PRIMARY KEY,
  "mr_link" TEXT NOT NULL,
  "merge_date" TEXT,
  "status" TEXT NOT NULL,
  "path" TEXT NOT NULL,
  "from_hash" TEXT NOT NULL,
  "to_hash" TEXT NOT NULL,
  "created_at" TEXT NOT NULL,
  "updated_at" TEXT NOT NULL
);
CREATE INDEX "idx_mr_path" ON "mega_mr" ("path");

CREATE TABLE IF NOT EXISTS "mega_mr_conv" (
  "id" INTEGER PRIMARY KEY,
  "mr_id" INTEGER NOT NULL,
  "user_id" INTEGER NOT NULL,
  "conv_type" TEXT NOT NULL,
  "created_at" TEXT NOT NULL,
  "updated_at" TEXT NOT NULL
);
CREATE INDEX "idx_conversation" ON "mega_mr_conv" ("mr_id");


CREATE TABLE IF NOT EXISTS "mega_mr_comment" (
  "id" INTEGER PRIMARY KEY,
  "conv_id" INTEGER NOT NULL,
  "comment" TEXT,
  "edited" INTEGER NOT NULL
);
CREATE INDEX "idx_comment_id" ON "mega_mr_comment" ("conv_id");

CREATE TABLE IF NOT EXISTS "mega_issue" (
  "id" INTEGER PRIMARY KEY,
  "number" INTEGER NOT NULL,
  "title" TEXT NOT NULL,
  "sender_name" TEXT NOT NULL,
  "sender_id" INTEGER NOT NULL,
  "state" TEXT NOT NULL,
  "created_at" TEXT NOT NULL,
  "updated_at" TEXT NOT NULL,
  "closed_at" TEXT DEFAULT NULL
);
CREATE TABLE IF NOT EXISTS "mega_refs" (
  "id" INTEGER PRIMARY KEY,
  "path" TEXT NOT NULL,
  "ref_commit_hash" TEXT NOT NULL,
  "ref_tree_hash" TEXT NOT NULL,
  "created_at" TEXT NOT NULL,
  "updated_at" TEXT NOT NULL,
  CONSTRAINT uniq_mref_path UNIQUE (path)
);
CREATE TABLE IF NOT EXISTS "import_refs" (
  "id" INTEGER PRIMARY KEY,
  "repo_id" INTEGER NOT NULL,
  "ref_name" TEXT NOT NULL,
  "ref_git_id" TEXT NOT NULL,
  "ref_type" TEXT NOT NULL,
  "default_branch" INTEGER NOT NULL,
  "created_at" TEXT NOT NULL,
  "updated_at" TEXT NOT NULL,
  CONSTRAINT uniq_ref_path_name UNIQUE (repo_id, ref_name)
);
CREATE INDEX "idx_refs_repo_id" ON "import_refs" ("repo_id");
CREATE TABLE IF NOT EXISTS "git_repo" (
  "id" INTEGER PRIMARY KEY,
  "repo_path" TEXT NOT NULL,
  "repo_name" TEXT NOT NULL,
  "created_at" TEXT NOT NULL,
  "updated_at" TEXT NOT NULL,
  CONSTRAINT uniq_ir_path UNIQUE (repo_path)
);
CREATE INDEX "idx_ir_repo_path" ON "git_repo" ("repo_path");
CREATE TABLE IF NOT EXISTS "git_commit" (
  "id" INTEGER PRIMARY KEY,
  "repo_id" INTEGER NOT NULL,
  "commit_id" TEXT NOT NULL,
  "tree" TEXT NOT NULL,
  "parents_id" TEXT NOT NULL,  -- Use JSON to store array
  "author" TEXT,
  "committer" TEXT,
  "content" TEXT,
  "created_at" TEXT NOT NULL,
  CONSTRAINT uniq_c_git_repo_id UNIQUE (repo_id, commit_id)
);
CREATE INDEX "idx_ic_git_id" ON "git_commit" ("commit_id");
CREATE INDEX "idx_ic_repo_id" ON "git_commit" ("repo_id");
CREATE TABLE IF NOT EXISTS "git_tree" (
  "id" INTEGER PRIMARY KEY,
  "repo_id" INTEGER NOT NULL,
  "tree_id" TEXT NOT NULL,
  "sub_trees" BLOB NOT NULL,
  "size" INTEGER NOT NULL,
  "commit_id" TEXT NOT NULL,
  "created_at" TEXT NOT NULL,
  CONSTRAINT uniq_t_git_repo UNIQUE (repo_id, tree_id)
);
CREATE INDEX "idx_t_git_id" ON "git_tree" ("tree_id");
CREATE INDEX "idx_t_repo_id" ON "git_tree" ("repo_id");
CREATE TABLE IF NOT EXISTS "git_blob" (
  "id" INTEGER PRIMARY KEY,
  "repo_id" INTEGER NOT NULL,
  "blob_id" TEXT NOT NULL,
  "name" TEXT,
  "size" INTEGER NOT NULL,
  "commit_id" TEXT NOT NULL,
  "created_at" TEXT NOT NULL,
  CONSTRAINT uniq_b_git_repo UNIQUE (repo_id, blob_id)
);
CREATE INDEX "idx_b_git_id" ON "git_blob" ("blob_id");
CREATE TABLE IF NOT EXISTS "git_tag" (
  "id" INTEGER PRIMARY KEY,
  "repo_id" INTEGER NOT NULL,
  "tag_id" TEXT NOT NULL,
  "object_id" TEXT NOT NULL,
  "object_type" TEXT NOT NULL,
  "tag_name" TEXT NOT NULL,
  "tagger" TEXT NOT NULL,
  "message" TEXT NOT NULL,
  "created_at" TEXT NOT NULL,
  CONSTRAINT uniq_gtag_tag_id UNIQUE (tag_id)
);
CREATE TABLE IF NOT EXISTS "raw_blob" (
  "id" INTEGER PRIMARY KEY,
  "sha1" TEXT NOT NULL,
  "content" TEXT,
  "file_type" TEXT,
  "storage_type" TEXT NOT NULL,
  "data" BLOB,
  "local_path" TEXT,
  "remote_url" TEXT,
  "created_at" TEXT NOT NULL,
  CONSTRAINT uniq_rb_sha1 UNIQUE (sha1)
);
CREATE INDEX "idx_rb_sha1" ON "raw_blob" ("sha1");
CREATE TABLE IF NOT EXISTS "git_pr" (
  "id" INTEGER PRIMARY KEY,
  "number" INTEGER NOT NULL,
  "title" TEXT NOT NULL,
  "state" TEXT NOT NULL,
  "created_at" TEXT NOT NULL,
  "updated_at" TEXT NOT NULL,
  "closed_at" TEXT DEFAULT NULL,
  "merged_at" TEXT DEFAULT NULL,
  "merge_commit_sha" TEXT DEFAULT NULL,
  "repo_id" INTEGER NOT NULL,
  "sender_name" TEXT NOT NULL,
  "sender_id" INTEGER NOT NULL,
  "user_name" TEXT NOT NULL,
  "user_id" INTEGER NOT NULL,
  "commits_url" TEXT NOT NULL,
  "patch_url" TEXT NOT NULL,
  "head_label" TEXT NOT NULL,
  "head_ref" TEXT NOT NULL,
  "base_label" TEXT NOT NULL,
  "base_ref" TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS "git_issue" (
  "id" INTEGER PRIMARY KEY,
  "number" INTEGER NOT NULL,
  "title" TEXT NOT NULL,
  "sender_name" TEXT NOT NULL,
  "sender_id" INTEGER NOT NULL,
  "state" TEXT NOT NULL,
  "created_at" TEXT NOT NULL,
  "updated_at" TEXT NOT NULL,
  "closed_at" TEXT DEFAULT NULL,
  "repo_id" INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS "lfs_locks" (
  "id" TEXT PRIMARY KEY,
  "data" TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS "lfs_objects" (
  "oid" TEXT PRIMARY KEY,
  "size" INTEGER NOT NULL,
  "exist" INTEGER NOT NULL,
  "splited" INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS "lfs_split_relations" (
  "ori_oid" TEXT NOT NULL,
  "sub_oid" TEXT NOT NULL,
  "offset" INTEGER NOT NULL,
  "size" INTEGER NOT NULL,
  PRIMARY KEY ("ori_oid", "sub_oid", "offset")
);


CREATE TABLE IF NOT EXISTS "ztm_node" (
  "peer_id" TEXT PRIMARY KEY,
  "hub" TEXT,
  "agent_name" TEXT,
  "service_name" TEXT,
  "type" TEXT,
  "online" INTEGER NOT NULL,
  "last_online_time" INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS "ztm_repo_info" (
  "identifier" TEXT PRIMARY KEY,
  "name" TEXT,
  "origin" TEXT,
  "update_time" INTEGER NOT NULL,
  "commit" TEXT
);

CREATE TABLE IF NOT EXISTS "mq_storage" (
  "id" INTEGER PRIMARY KEY,
  "category" TEXT,
  "create_time" TIMESTAMP NOT NULL,
  "content" TEXT
);
