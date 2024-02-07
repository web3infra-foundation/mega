

## 1.Basic Design of Mega Monorepo

The purpose of this document is to refactor the current storage design of Mega, enabling it to effectively manage project monorepos while remaining compatible with the Git protocol.

Mega's storage structure is mainly divided into the following parts:

### Mega Directory: 

Similar to the 'tree' in Git, Mega maintains relationships between files and file names. In the database, Mega independently manages directory information for the current version.

### b-link file*
The purpose of the B-link file is to store file index information, serving as a replacement for blobs in Git. The design of this structure is inspired by the specification of Git LFS, as follows:
  ```bash
  version https://mega.com/directory/spec/v1
  blob 3a739f77180d81aa45d9bd11eb6be7098bf1991f
  ```
It includes the following records:
- version: Represents the version information of the structure.
- blob: Points to the hash value of the actual blob.

### Import directory
- The primary purpose of importing directories is to synchronize the original Git repository into the Mega directory. Projects within the import directory are maintained in a **read-only** state, preserving the original commit information.
- Projects pushed to the import directory can have multiple commits.
- Projects in the import directory can be quickly transformed into the Mega directory.
- Import directories can be configured in the configuration file.
- Once a directory is initialized as an import directory, it cannot be changed back to a regular directory.

## 2. Database Design

### Table Overall

| Table Name    | Description                                                                                             | Mega Push | Mega Pull | Git Push | Git Repo |
| ------------- | ------------------------------------------------------------------------------------------------------- | --------- | --------- | -------- | -------- |
| mega_snapshot | Mainain the latest directory stucture in monorepo.                                                      | &#10003;  | &#10003;  |          |          |
| mega_commit   | Store all commit objects related with mega directory, have mr status                                    | &#10003;  |           |          |          |
| mega_tree     | Store all tree objects related with mega directory, together with mega_commit to find history directory | &#10003;  |           |          |          |
| mega_blob     | Store all blob objects under mega directory.                                                            | &#10003;  | &#10003;  |          |          |
| mega_tag      | Store all annotated tag with mega directory.                                                            | &#10003;  | &#10003;  |          |          |
| mega_mr       | Merge request related to mega commits.                                                                  | &#10003;  |           |          |          |
| mega_issue    | Manage mega's issue.                                                                                    |           |           |          |          |
| git_repo      | Maintain Relations between impoprt_repo and repo_path.                                                  |           |           | &#10003; | &#10003; |
| git_refs      | Obtains the latest commit_id through repo_id and ref_name, also storing the repo lightweight tags.      |           |           | &#10003; | &#10003; |
| git_commit    | Store all parsed commit objects related with repo.                                                      |           |           | &#10003; | &#10003; |
| git_tree      | Store all parsed tree objects related with repo.                                                        |           |           | &#10003; | &#10003; |
| git_blob      | Store all parsed blob objects related with repo.                                                        |           |           | &#10003; | &#10003; |
| git_tag       | Store all annotated tag related with repo.                                                              |           |           | &#10003; | &#10003; |
| raw_objects   | Store all raw objects with both git repo and mega directory.                                            | &#10003;  | &#10003;  | &#10003; | &#10003; |
| git_pr        | Pull request sync from third parties like GitHub.                                                       |           |           |          |          |
| git_issue     | Issues sync from third parties like GitHub.                                                             |           |           |          |          |
| lfs_objects   | Store objects related to LFS protocol.                                                                  |           |           |          |          |
| lfs_locks     | Store locks for lfs files.                                                                              |           |           |          |          |

### ER Diagram


  ```mermaid
  ---
  title: Mgea ER Diagram
  ---
%%{init: {"theme": "default", "themeCSS": ["[id*=m] .er.entityBox { fill: orange;}"]}}%%

erDiagram
    msnap["MEGA-SNAPSHOT"] mc["MEGA-COMMITS"] mt["MEGA-TREE"] mb["MEGA-BLOB"] mtag["MEGA-TAG"] mmr["MEGA-MR"]
    grp["GIT-REPO"] grf["GIT-REFS"] gc["GIT-COMMIT"] gt["GIT-TREE"] gb["GIT-BLOB"] gtag["GIT-TAG"] gp["GIT-PR"] gi["GIT-ISSUE"]
    raw["RAW-OBJETCS"]
    lo["LFS-OBJECTS"] lk["LFS-LOCKS"]

    msnap |o--|{ mc : "belong to"
    msnap |o--|{ gt : contains
    mc ||--|| mt : points
    mc ||--|| raw : points
    mc ||--|| mmr : "belong to"
    mt }|--o{ mb : points
    mt ||--|| raw : points
    mt }|--|| mmr : "belong to"
    mt }o..o{ gt : points
    mb ||--|| raw : points
    mb }|--|| mmr : "belong to"
    mtag |o--o| mc : points
    mtag ||--|| raw : points
    raw ||--o| lo : points
    lo ||--o| lk : points
    gp }o--|| grp : "belong to"
    gi }o--|| grp : "belong to"
    grf ||--|| gc : points
    grf ||--|| gtag : points
    grf }|--|| grp : "belong to"
    gc ||--|| gt : has
    gc ||--|| raw : has
    gc }|--|| grp : "belong to"
    gt ||--o{ gb : has
    gt ||--|| raw : points
    gt }|--|| grp : "belong to"
    gb ||--|| raw : points
    gb }|--|| grp : "belong to"
    gtag }o--|| grp : "belong to"
    gtag |o--o| gc : points
    gtag ||--|| raw : points

  ```

### Table Details


#### mega_snapshot

| Column     | Type        | Constraints | Description                                              |
| ---------- | ----------- | ----------- | -------------------------------------------------------- |
| id         | BIGINT      | PRIMARY KEY |                                                          |
| path       | TEXT        | NOT NULL    |                                                          |
| import_dir | BOOLEAN     | NOT NULL    | points to git_tree if is import_dir, else points to self |
| tree_id    | VARCHAR(40) | NOT NULL    |                                                          |
| sub_trees  | TEXT[]      |             | {name, sha1, mode, repo_id}                              |
| commit_id  | VARCHAR(40) | NOT NULL    | the latest commit related to this directory              |
| size       | INT         | NOT NULL    | used for count file size under directory                 |
| created_at | TIMESTAMP   | NOT NULL    |                                                          |
| updated_at | TIMESTAMP   | NOT NULL    |                                                          |


#### mega_commit

| Column     | Type        | Constraints | Description                                     |
| ---------- | ----------- | ----------- | ----------------------------------------------- |
| id         | BIGINT      | PRIMARY KEY |                                                 |
| commit_id  | VARCHAR(40) | NOT NULL    |                                                 |
| tree       | VARCHAR(40) | NOT NULL    |                                                 |
| parents_id | TEXT[]      |             |                                                 |
| author     | TEXT        |             |                                                 |
| committer  | TEXT        |             |                                                 |
| content    | TEXT        |             |                                                 |
| mr_id      | VARCHAR(20) |             |                                                 |
| status     | VARCHAR(20) | NOT NULL    | mr satus, might be 'Open','Merged' and 'Closed' |
| size       | INT         | NOT NULL    | used for magic sort in pack process             |
| full_path  | TEXT        | NOT NULL    | used for magic sort in pack process             |
| created_at | TIMESTAMP   | NOT NULL    |                                                 |
| updated_at | TIMESTAMP   | NOT NULL    |                                                 |


#### mega_tree

| Column     | Type        | Constraints | Description                          |
| ---------- | ----------- | ----------- | ------------------------------------ |
| id         | BIGINT      | PRIMARY KEY |                                      |
| tree_id    | VARCHAR(40) | NOT NULL    |                                      |
| sub_trees  | TEXT[]      |             | {name, sha1, mode, repo_id}          |
| import_dir | BOOLEAN     | NOT NULL    | point to git_tree if it's import dir |
| mr_id      | VARCHAR(20) |             |                                      |
| status     | VARCHAR(20) | NOT NULL    |                                      |
| size       | INT         | NOT NULL    |                                      |
| full_path  | TEXT        | NOT NULL    |                                      |
| created_at | TIMESTAMP   | NOT NULL    |                                      |
| updated_at | TIMESTAMP   | NOT NULL    |                                      |

#### mega_blob

| Column       | Type        | Constraints |
| ------------ | ----------- | ----------- |
| id           | BIGINT      | PRIMARY KEY |
| blob_id      | VARCHAR(40) | NOT NULL    |
| commit_id    | VARCHAR(40) | NOT NULL    |
| mr_id        | VARCHAR(20) |             |
| status       | VARCHAR(20) | NOT NULL    |
| size         | INT         | NOT NULL    |
| content      | TEXT        | NOT NULL    |
| content_type | VARCHAR(20) |             |
| full_path    | TEXT        | NOT NULL    |
| created_at   | TIMESTAMP   | NOT NULL    |
| updated_at   | TIMESTAMP   | NOT NULL    |


#### mega_tag

| Column      | Type        | Constraints | Description                                                 |
| ----------- | ----------- | ----------- | ----------------------------------------------------------- |
| id          | BIGINT      | PRIMARY KEY |                                                             |
| tag_id      | VARCHAR(40) | NOT NULL    |                                                             |
| object_id   | VARCHAR(40) | NOT NULL    | point to the object's sha1                                  |
| object_type | VARCHAR(20) | NOT NULL    | In Git, each object type is assigned a unique integer value |
| tag_name    | TEXT        | NOT NULL    | tag's name                                                  |
| tagger      | TEXT        | NOT NULL    | tag's signature                                             |
| message     | TEXT        | NOT NULL    |                                                             |
| created_at  | TIMESTAMP   | NOT NULL    |                                                             |

#### mega_mr

| Column     | Type         | Constraints | Description                                      |
| ---------- | ------------ | ----------- | ------------------------------------------------ |
| id         | BIGINT       | PRIMARY KEY |                                                  |
| mr_link    | VARCHAR(40)  | NOT NULL    | A MR identifier with a length of 6-8 characters. |
| mr_msg     | VARCHAR(255) | NOT NULL    |                                                  |
| merge_date | TIMESTAMP    | NOT NULL    |                                                  |
| status     | VARCHAR(20)  | NOT NULL    |                                                  |
| created_at | TIMESTAMP    | NOT NULL    |                                                  |
| updated_at | TIMESTAMP    | NOT NULL    |                                                  |

#### mega_issue

| Column      | Type         | Constraints  |
| ----------- | ------------ | ------------ |
| id          | BIGINT       | PRIMARY KEY  |
| number      | BIGINT       | NOT NULL     |
| title       | VARCHAR(255) | NOT NULL     |
| sender_name | VARCHAR(255) | NOT NULL     |
| sender_id   | BIGINT       | NOT NULL     |
| state       | VARCHAR(255) | NOT NULL     |
| created_at  | TIMESTAMP    | NOT NULL     |
| updated_at  | TIMESTAMP    | NOT NULL     |
| closed_at   | TIMESTAMP    | DEFAULT NULL |

#### git_refs

| Column     | Type        | Constraints | Description                           |
| ---------- | ----------- | ----------- | ------------------------------------- |
| id         | BIGINT      | PRIMARY KEY |                                       |
| repo_id    | BIGINT      | NOT NULL    |                                       |
| ref_name   | TEXT        | NOT NULL    | reference name, can be branch and tag |
| ref_git_id | VARCHAR(40) | NOT NULL    | point to the commit or tag object     |
| is_commit  | BOOLEAN     | NOT NULL    | set true if point to a commit         |
| created_at | TIMESTAMP   | NOT NULL    |                                       |
| updated_at | TIMESTAMP   | NOT NULL    |                                       |


#### git_repo

| Column     | Type      | Constraints | Description                                   |
| ---------- | --------- | ----------- | --------------------------------------------- |
| id         | BIGINT    | PRIMARY KEY |                                               |
| repo_path  | TEXT      | NOT NULL    | git repo's absulote path under mega directory |
| created_at | TIMESTAMP | NOT NULL    |                                               |
| updated_at | TIMESTAMP | NOT NULL    |                                               |

#### git_commit

| Column     | Type        | Constraints |
| ---------- | ----------- | ----------- |
| id         | BIGINT      | PRIMARY KEY |
| repo_id    | BIGINT      | NOT NULL    |
| commit_id  | VARCHAR(40) | NOT NULL    |
| tree       | VARCHAR(40) | NOT NULL    |
| pid        | TEXT[]      |             |
| author     | TEXT        |             |
| committer  | TEXT        |             |
| content    | TEXT        |             |
| size       | INT         | NOT NULL    |
| full_path  | TEXT        | NOT NULL    |
| created_at | TIMESTAMP   | NOT NULL    |

#### git_tree

| Column     | Type         | Constraints |
| ---------- | ------------ | ----------- |
| id         | BIGINT       | PRIMARY KEY |
| repo_id    | BIGINT       | NOT NULL    |
| tree_id    | VARCHAR(40)  | NOT NULL    |
| sub_trees  | TEXT[]       |             |
| name       | VARCHAR(128) |             |
| size       | INT          | NOT NULL    |
| full_path  | TEXT         | NOT NULL    |
| commit_id  | VARCHAR(40)  | NOT NULL    |
| created_at | TIMESTAMP    | NOT NULL    |

#### git_blob

| Column       | Type         | Constraints |
| ------------ | ------------ | ----------- |
| id           | BIGINT       | PRIMARY KEY |
| repo_id      | BIGINT       | NOT NULL    |
| blob_id      | VARCHAR(40)  | NOT NULL    |
| name         | VARCHAR(128) |             |
| size         | INT          | NOT NULL    |
| content      | TEXT         | NOT NULL    |
| content_type | VARCHAR(20)  |             |
| full_path    | TEXT         | NOT NULL    |
| commit_id    | VARCHAR(40)  | NOT NULL    |
| created_at   | TIMESTAMP    | NOT NULL    |

#### git_tag

| Column      | Type        | Constraints |
| ----------- | ----------- | ----------- |
| id          | BIGINT      | PRIMARY KEY |
| repo_id     | BIGINT      | NOT NULL    |
| tag_id      | VARCHAR(40) | NOT NULL    |
| object_id   | VARCHAR(40) | NOT NULL    |
| object_type | VARCHAR(20) | NOT NULL    |
| tag_name    | TEXT        | NOT NULL    |
| tagger      | TEXT        | NOT NULL    |
| message     | TEXT        | NOT NULL    |
| created_at  | TIMESTAMP   | NOT NULL    |

#### raw_objects

| Column             | Type        | Constraints | Description                                                             |
| ------------------ | ----------- | ----------- | ----------------------------------------------------------------------- |
| id                 | BIGINT      | PRIMARY KEY |                                                                         |
| sha1               | VARCHAR(40) | NOT NULL    | git object's sha1 hash                                                  |
| object_type        | VARCHAR(20) | NOT NULL    |                                                                         |
| storage_type       | INT         | NOT NULL    | data storage type, can be 0-database; 1-local file system; 2-remote url |
| data               | BYTEA       |             |                                                                         |
| local_storage_path | TEXT        |             |                                                                         |
| remote_url         | TEXT        |             |                                                                         |


#### git_pr

| Column           | Type         | Constraints  |
| ---------------- | ------------ | ------------ |
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
| ----------- | ------------ | ------------ |
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
| ------ | ----------- | ----------- |
| id     | VARCHAR(40) | PRIMARY KEY |
| data   | TEXT        |             |


#### lfs_objects

| Column | Type        | Constraints |
| ------ | ----------- | ----------- |
| oid    | VARCHAR(64) | PRIMARY KEY |
| size   | BIGINT      |             |
| exist  | BOOLEAN     |             |


## 3. Sql execution for each process.


#### Use mega init command to initialize mega directory: 

- Init commit points to tree:
    ```sql
    insert into mega_commit values (...);
    ```
- Build directory and tree objs:
    ```sql
    insert into mega_snapshot values ('/root', false, ...);
    insert into mega_snapshot values ('/root/projects', false, ...);
    insert into mega_snapshot values ('/root/import', true, ...);
    insert into mega_snapshot values ('/root/projects/rust', false, ...);
    insert into mega_tree values (...);
    ```
- Generate ReadMe.md file and insert to raw_objects:
    ```sql
    insert into mega_blob values (id, blob_id, 1024, 0, 'Merged');
    insert into raw_objects values (...);
    ```


#### Clone mega directory

- Check path is import directory
    ```sql
    select * from mega_snapshot where path = '/path/to/directory';
    ```
- If it's a mega directory

  - Check clone object size exceed the threshold:
    ```sql
    <!-- get all files under path -->
    select * from mega_snapshot where tree_id in (...);
    <!-- calculate objects size with all blob ids -->
    select * from mega_blob where blob_id in (...);
    ```
  - Construct new commit
    ```sql
    <!-- get related commit -->
    select * from mega_commit where commit_id = ?;
    ```
  - Pack file with new commit and raw tree and objects;
    ```sql
    <!-- get related trees and objects -->
    select * from raw_objects where sha1 in (...);
    ```

- Or a import directory(see clone a repo)

#### Push back mega directory
- Parse packfile get trees and objs
  ```sql
  select * from mega_snapshot where path = '/path/to/directory';
  ```
- Open new merge request
  ```sql
  insert into raw_objects values(...);
  insert into merge_request values(...);
  insert into mega_tree values(..., 'Open');
  insert into mega_commit values(..., 'Open');
  insert into mega_blob values(..., 'Open');
  ```
- Merge Request
  ```sql
  update merge_request set status = 'Merged';
  update mega_tree set status = 'Merged';
  update mega_commit set status = 'Merged';
  update mega_blob set status = 'Merged';
  update mega_snapshot set (commit_id, sub_trees) where path = ?;
  ```

#### Clone repo 
  - Find related objects
    ```sql
    select * from git_repo where repo_path = '/path/to/repo'
    select commmit_id from git_commit where repo_id = ?;
    select tree_id from git_tree where repo_id = ?;
    select blob_id from git_blob where repo_id = ?;
    ```
  - Find raw objects by id
  ```sql
  select * from raw_objects where repo_id =? and sha1 in (...);
  ```  
  - Pack file with raw_objetcs


#### Push back repo
- Check server refs by path
  ```sql
  select * from git_repo where repo_path = '/path/to/repo'
  select * from git_refs where repo_id = ...;
  ```
- Parse pack file and save objects
  ```sql
  insert into raw_objectss values(...);
  <!-- convert raw_obj to objects -->
  ```
- If under import directory
  ```sql
  insert into git_commit values (c1),(c2),(c3);
  insert into git_tree values (T1)...(T4);
  insert into git_blob values (B1)...(B5);
  ```
- Update refs
  ```sql
  update git_refs set ref_git_id = ? where repo_id =?;
  ```

## 4. Prerequisites

- You need to execute SQL files in a specific order to init the database.

    For example using `PostgreSQL`, execute the files under `sql\postgres`:

        pg_20230803__init.sql

    or if your are using `Mysql`, execute scripts:

        mysql_20230523__init.sql



- Generating entities: 
Entities can be generated from the database table structure with the following command

`sea-orm-cli generate entity -u "postgres://${DB_USERNAME}:${DB_SECRET}@${DB_HOST}/mega"  -o database/entity/src` 