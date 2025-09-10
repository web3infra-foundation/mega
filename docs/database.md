

## 1.Basic Design of Mega Monorepo

The purpose of this document is to refactor the current storage design of Mega, enabling it to effectively manage project monorepo while remaining compatible with the Git protocol.

Mega's storage structure is mainly divided into the following parts:

### Mega Directory: 

Similar to the 'tree' in Git, Mega maintains relationships between files and file names. In the database, Mega independently manages directory information for the current version.

### Import directory
- The primary purpose of importing directories is to synchronize the original Git repository into the Mega directory. Projects within the import directory are maintained in a **read-only** state, preserving the original commit information.
- Projects pushed to the import directory can have multiple commits.
- Projects in the import directory can be quickly transformed into the Mega directory.
- Import directories can be configured in the configuration file.
- Once a directory is initialized as an import directory, it cannot be changed back to a regular directory.

## 2. Database Design

### Table Overall

| Table Name      | Description                                                                                             | Mega Push | Mega Pull | Git Push | Git Repo |
|-----------------|---------------------------------------------------------------------------------------------------------|-----------|-----------|----------|----------|
| mega_commit     | Store all commit objects related with mega directory, have mr status                                    | &#10003;  |           |          |          |
| mega_tree       | Store all tree objects related with mega directory, together with mega_commit to find history directory | &#10003;  |           |          |          |
| mega_blob       | Store all blob objects under mega directory.                                                            | &#10003;  | &#10003;  |          |          |
| mega_tag        | Store all annotated tag with mega directory.                                                            | &#10003;  | &#10003;  |          |          |
| mega_mr         | Merge request related to mega commits.                                                                  | &#10003;  |           |          |          |
| mega_mr_conv    | MR conversation list                                                                                    | &#10003;  |           |          |          |
| mega_mr_comment | MR Comment                                                                                              | &#10003;  |           |          |          |
| mega_issue      | Manage mega's issue.                                                                                    |           |           |          |          |
| mega_refs       | This table maintains refs information corresponding to each directory of mega                           | &#10003;  |           |          |          |
| git_repo        | Maintain Relations between import_repo and repo_path.                                                   |           |           | &#10003; | &#10003; |
| import_refs     | Obtains the latest commit_id through repo_id and ref_name, also storing the repo lightweight tags.      |           |           | &#10003; | &#10003; |
| git_commit      | Store all parsed commit objects related with repo.                                                      |           |           | &#10003; | &#10003; |
| git_tree        | Store all parsed tree objects related with repo.                                                        |           |           | &#10003; | &#10003; |
| git_blob        | Store all parsed blob objects related with repo.                                                        |           |           | &#10003; | &#10003; |
| git_tag         | Store all annotated tag related with repo.                                                              |           |           | &#10003; | &#10003; |
| raw_blob        | Store all raw objects with both git repo and mega directory.                                            | &#10003;  | &#10003;  | &#10003; | &#10003; |
| git_pr          | Pull request sync from third parties like GitHub.                                                       |           |           |          |          |
| git_issue       | Issues sync from third parties like GitHub.                                                             |           |           |          |          |
| lfs_objects     | Store objects related to LFS protocol.                                                                  |           |           |          |          |
| lfs_locks       | Store locks for lfs files.                                                                              |           |           |          |          |
| commit_auths    | Store commit binding information associating commits with users.                                         | &#10003;  |           |          |          |

### ER Diagram


```mermaid
erDiagram
    
    MEGA-SNAPSHOT |o--|{ MEGA-COMMITS : "belong to"
    MEGA-SNAPSHOT |o--|{ GIT-TREE : contains
    MEGA-COMMITS ||--|| MEGA-TREE : points
    MEGA-COMMITS ||--|| RAW-OBJECTS : points
    MEGA-COMMITS ||--|| MEGA-MR : "belong to"
    MEGA-TREE }|--o{ MEGA-BLOB : points
    MEGA-TREE ||--|| RAW-OBJECTS : points
    MEGA-TREE }|--|| MEGA-MR : "belong to"
    MEGA-TREE }o..o{ GIT-TREE : points
    MEGA-BLOB ||--|| RAW-OBJECTS : points
    MEGA-BLOB }|--|| MEGA-MR : "belong to"
    MEGA-TAG |o--o| MEGA-COMMITS : points
    MEGA-TAG ||--|| RAW-OBJECTS : points
    RAW-OBJECTS ||--o| LFS-OBJECTS : points
    LFS-OBJECTS ||--o| LFS-LOCKS : points
    GIT-PR }o--|| GIT-REPO : "belong to"
    GIT-ISSUE }o--|| GIT-REPO : "belong to"
    GIT-REFS ||--|| GIT-COMMIT : points
    GIT-REFS ||--|| GIT-TAG : points
    GIT-REFS }|--|| GIT-REPO : "belong to"
    GIT-COMMIT ||--|| GIT-TREE : has
    GIT-COMMIT ||--|| RAW-OBJECTS : has
    GIT-COMMIT }|--|| GIT-REPO : "belong to"
    GIT-TREE ||--o{ GIT-BLOB : has
    GIT-TREE ||--|| RAW-OBJECTS : points
    GIT-TREE }|--|| GIT-REPO : "belong to"
    GIT-BLOB ||--|| RAW-OBJECTS : points
    GIT-BLOB }|--|| GIT-REPO : "belong to"
    GIT-TAG }o--|| GIT-REPO : "belong to"
    GIT-TAG |o--o| GIT-COMMIT : points
    GIT-TAG ||--|| RAW-OBJECTS : points
    COMMIT-AUTHS ||--o| GIT-COMMIT : binds
    COMMIT-AUTHS ||--o| MEGA-COMMITS : binds

```

### Table Details


#### mega_commit

| Column     | Type        | Constraints | Description |
|------------|-------------|-------------|-------------|
| id         | BIGINT      | PRIMARY KEY |             |
| commit_id  | VARCHAR(40) | NOT NULL    |             |
| tree       | VARCHAR(40) | NOT NULL    |             |
| parents_id | TEXT[]      | NOT NULL    |             |
| author     | TEXT        |             |             |
| committer  | TEXT        |             |             |
| content    | TEXT        |             |             |
| created_at | TIMESTAMP   | NOT NULL    |             |


#### mega_tree

| Column     | Type        | Constraints | Description                 |
|------------|-------------|-------------|-----------------------------|
| id         | BIGINT      | PRIMARY KEY |                             |
| tree_id    | VARCHAR(40) | NOT NULL    |                             |
| sub_trees  | TEXT[]      | NOT NULL    | {name, sha1, mode, repo_id} |
| commit_id  | VARCHAR(40) | NOT NULL    |                             |
| size       | INT         | NOT NULL    |                             |
| created_at | TIMESTAMP   | NOT NULL    |                             |

#### mega_blob

| Column     | Type        | Constraints |
|------------|-------------|-------------|
| id         | BIGINT      | PRIMARY KEY |
| blob_id    | VARCHAR(40) | NOT NULL    |
| commit_id  | VARCHAR(40) | NOT NULL    |
| name       | TEXT        | NOT NULL    |
| size       | INT         | NOT NULL    |
| created_at | TIMESTAMP   | NOT NULL    |


#### mega_tag

| Column      | Type        | Constraints | Description                                                 |
|-------------|-------------|-------------|-------------------------------------------------------------|
| id          | BIGINT      | PRIMARY KEY |                                                             |
| tag_id      | VARCHAR(40) | NOT NULL    |                                                             |
| object_id   | VARCHAR(40) | NOT NULL    | point to the object's sha1                                  |
| object_type | VARCHAR(20) | NOT NULL    | In Git, each object type is assigned a unique integer value |
| tag_name    | TEXT        | NOT NULL    | tag's name                                                  |
| tagger      | TEXT        | NOT NULL    | tag's signature                                             |
| message     | TEXT        | NOT NULL    |                                                             |
| created_at  | TIMESTAMP   | NOT NULL    |                                                             |

#### mega_mr

| Column     | Type        | Constraints | Description                                      |
|------------|-------------|-------------|--------------------------------------------------|
| id         | BIGINT      | PRIMARY KEY |                                                  |
| mr_link    | VARCHAR(40) | NOT NULL    | A MR identifier with a length of 6-8 characters. |
| merge_date | TIMESTAMP   |             |                                                  |
| status     | VARCHAR(20) | NOT NULL    |                                                  |
| path       | TEXT        | NOT NULL    |                                                  |
| from_hash  | VARCHAR(40) | NOT NULL    | merge from which commit                          |
| to_hash    | VARCHAR(40) | NOT NULL    | merge to commit hash                             |
| created_at | TIMESTAMP   | NOT NULL    |                                                  |
| updated_at | TIMESTAMP   | NOT NULL    |                                                  |

#### mega_mr_conv

| Column     | Type        | Constraints | Description                                                      |
|------------|-------------|-------------|------------------------------------------------------------------|
| id         | BIGINT      | PRIMARY KEY |                                                                  |
| mr_id      | BIGINT      | NOT NULL    |                                                                  |
| user_id    | BIGINT      | NOT NULL    |                                                                  |
| conv_type  | VARCHAR(20) | NOT NULL    | conversation type, can be comment, commit, force push, edit etc. |
| created_at | TIMESTAMP   | NOT NULL    |                                                                  |
| updated_at | TIMESTAMP   | NOT NULL    |                                                                  |

#### mega_mr_comment

| Column  | Type    | Constraints | Description                     |
|---------|---------|-------------|---------------------------------|
| id      | BIGINT  | PRIMARY KEY |                                 |
| conv_id | BIGINT  | NOT NULL    | related table mega_mr_conv's id |
| comment | TEXT    |             |                                 |
| edited  | BOOLEAN | NOT NULL    |                                 |

#### mega_issue

| Column      | Type         | Constraints  |
|-------------|--------------|--------------|
| id          | BIGINT       | PRIMARY KEY  |
| number      | BIGINT       | NOT NULL     |
| title       | VARCHAR(255) | NOT NULL     |
| sender_name | VARCHAR(255) | NOT NULL     |
| sender_id   | BIGINT       | NOT NULL     |
| state       | VARCHAR(255) | NOT NULL     |
| created_at  | TIMESTAMP    | NOT NULL     |
| updated_at  | TIMESTAMP    | NOT NULL     |
| closed_at   | TIMESTAMP    | DEFAULT NULL |


#### mega_refs

| Column          | Type        | Constraints | Description                       |
|-----------------|-------------|-------------|-----------------------------------|
| id              | BIGINT      | PRIMARY KEY |                                   |
| path            | TEXT        | NOT NULL    | monorepo path refs                |
| ref_commit_hash | VARCHAR(40) | NOT NULL    | point to the commit or tag object |
| ref_tree_hash   | VARCHAR(40) | NOT NULL    | point to the tree object          |
| created_at      | TIMESTAMP   | NOT NULL    |                                   |
| updated_at      | TIMESTAMP   | NOT NULL    |                                   |


#### import_refs

| Column         | Type        | Constraints | Description                                      |
|----------------|-------------|-------------|--------------------------------------------------|
| id             | BIGINT      | PRIMARY KEY |                                                  |
| repo_id        | BIGINT      | NOT NULL    |                                                  |
| ref_name       | TEXT        | NOT NULL    | reference name, can be branch and tag            |
| ref_git_id     | VARCHAR(40) | NOT NULL    | point to the commit or tag object                |
| default_branch | BOOLEAN     | NOT NULL    | Indicates whether the branch is a default branch |
| ref_type       | VARCHAR(20) | NOT NULL    | ref_type: can be 'tag' or 'branch'               |
| created_at     | TIMESTAMP   | NOT NULL    |                                                  |
| updated_at     | TIMESTAMP   | NOT NULL    |                                                  |


#### git_repo

| Column     | Type      | Constraints | Description                                   |
|------------|-----------|-------------|-----------------------------------------------|
| id         | BIGINT    | PRIMARY KEY |                                               |
| repo_path  | TEXT      | NOT NULL    | git repo's absolute path under mega directory |
| repo_name  | TEXT      | NOT NULL    |                                               |
| created_at | TIMESTAMP | NOT NULL    |                                               |
| updated_at | TIMESTAMP | NOT NULL    |                                               |

#### git_commit

| Column     | Type        | Constraints |
|------------|-------------|-------------|
| id         | BIGINT      | PRIMARY KEY |
| repo_id    | BIGINT      | NOT NULL    |
| commit_id  | VARCHAR(40) | NOT NULL    |
| tree       | VARCHAR(40) | NOT NULL    |
| parents_id | TEXT[]      | NOT NULL    |
| author     | TEXT        |             |
| committer  | TEXT        |             |
| content    | TEXT        |             |
| created_at | TIMESTAMP   | NOT NULL    |

#### git_tree

| Column     | Type         | Constraints |
|------------|--------------|-------------|
| id         | BIGINT       | PRIMARY KEY |
| repo_id    | BIGINT       | NOT NULL    |
| tree_id    | VARCHAR(40)  | NOT NULL    |
| sub_trees  | TEXT[]       |             |
| size       | INT          | NOT NULL    |
| commit_id  | VARCHAR(40)  | NOT NULL    |
| created_at | TIMESTAMP    | NOT NULL    |

#### git_blob

| Column       | Type         | Constraints |
|--------------|--------------|-------------|
| id           | BIGINT       | PRIMARY KEY |
| repo_id      | BIGINT       | NOT NULL    |
| blob_id      | VARCHAR(40)  | NOT NULL    |
| name         | VARCHAR(128) |             |
| size         | INT          | NOT NULL    |
| commit_id    | VARCHAR(40)  | NOT NULL    |
| created_at   | TIMESTAMP    | NOT NULL    |

#### git_tag

| Column      | Type        | Constraints |
|-------------|-------------|-------------|
| id          | BIGINT      | PRIMARY KEY |
| repo_id     | BIGINT      | NOT NULL    |
| tag_id      | VARCHAR(40) | NOT NULL    |
| object_id   | VARCHAR(40) | NOT NULL    |
| object_type | VARCHAR(20) | NOT NULL    |
| tag_name    | TEXT        | NOT NULL    |
| tagger      | TEXT        | NOT NULL    |
| message     | TEXT        | NOT NULL    |
| created_at  | TIMESTAMP   | NOT NULL    |

#### raw_blob

| Column       | Type        | Constraints | Description                                                       |
|--------------|-------------|-------------|-------------------------------------------------------------------|
| id           | BIGINT      | PRIMARY KEY |                                                                   |
| sha1         | VARCHAR(40) | NOT NULL    | git object's sha1 hash                                            |
| content      | TEXT        |             |
| file_type    | VARCHAR(20) |             |
| storage_type | VARCHAR(20) | NOT NULL    | data storage type, can be 'database', 'local-fs' and 'remote_url' |
| data         | BYTEA       |             |                                                                   |
| local_path   | TEXT        |             |                                                                   |
| remote_url   | TEXT        |             |                                                                   |
| created_at   | TIMESTAMP   | NOT NULL    |

#### git_pr

| Column           | Type         | Constraints  |
|------------------|--------------|--------------|
| id               | BIGINT       | PRIMARY KEY  |
| number           | BIGINT       | NOT NULL     |
| title            | VARCHAR(255) | NOT NULL     |
| state            | VARCHAR(255) | NOT NULL     |
| created_at       | TIMESTAMP    | NOT NULL     |
| updated_at       | TIMESTAMP    | NOT NULL     |
| closed_at        | TIMESTAMP    | DEFAULT NULL |
| merged_at        | TIMESTAMP    | DEFAULT NULL |
| merge_commit_sha | VARCHAR(200) | DEFAULT NULL |
| repo_id          | BIGINT       | NOT NULL     |
| sender_name      | VARCHAR(255) | NOT NULL     |
| sender_id        | BIGINT       | NOT NULL     |
| user_name        | VARCHAR(255) | NOT NULL     |
| user_id          | BIGINT       | NOT NULL     |
| commits_url      | VARCHAR(255) | NOT NULL     |
| patch_url        | VARCHAR(255) | NOT NULL     |
| head_label       | VARCHAR(255) | NOT NULL     |
| head_ref         | VARCHAR(255) | NOT NULL     |
| base_label       | VARCHAR(255) | NOT NULL     |
| base_ref         | VARCHAR(255) | NOT NULL     |


#### git_issue

| Column      | Type         | Constraints  |
|-------------|--------------|--------------|
| id          | BIGINT       | PRIMARY KEY  |
| number      | BIGINT       | NOT NULL     |
| title       | VARCHAR(255) | NOT NULL     |
| sender_name | VARCHAR(255) | NOT NULL     |
| sender_id   | BIGINT       | NOT NULL     |
| state       | VARCHAR(255) | NOT NULL     |
| created_at  | TIMESTAMP    | NOT NULL     |
| updated_at  | TIMESTAMP    | NOT NULL     |
| closed_at   | TIMESTAMP    | DEFAULT NULL |
| repo_id     | BIGINT       | NOT NULL     |


#### lfs_locks

| Column | Type        | Constraints |
|--------|-------------|-------------|
| id     | VARCHAR(40) | PRIMARY KEY |
| data   | TEXT        |             |


#### lfs_objects

| Column | Type        | Constraints |
|--------|-------------|-------------|
| oid    | VARCHAR(64) | PRIMARY KEY |
| size   | BIGINT      |             |
| exist  | BOOLEAN     |             |


#### commit_auths

| Column           | Type        | Constraints | Description                                      |
|------------------|-------------|-------------|--------------------------------------------------|
| id               | VARCHAR     | PRIMARY KEY |                                                  |
| commit_sha       | VARCHAR     | NOT NULL    | Git commit SHA hash                              |
| author_email     | VARCHAR     | NOT NULL    | Original author email from commit                |
| matched_username  | VARCHAR     | NULL        | Username that this commit is bound to             |
| is_anonymous     | BOOLEAN     | NOT NULL    | Whether the commit should be treated as anonymous |
| matched_at       | TIMESTAMP   | NULL        | When the binding was created/updated             |
| created_at       | TIMESTAMP   | NOT NULL    | When the record was created                      |


## 3. Prerequisites


- Generating entities: 
Entities can be generated from the database table structure with the following command

`sea-orm-cli generate entity -u "postgres://postgres:$postgres@localhost/mega"  -o jupiter/entity/src` 