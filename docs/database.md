

## 1.Basic Design of Mega Monorepo

![Mega Directory Design](images/mega-tree-directory.png)

本文档的目的是为了重构mega目前的存储设计，使得mega既能实现项目monorepo管理，又能兼容git协议

Mega的存储结主要分为以下几部分:

### Mega Directory: 
类似于git中的tree，维护了文件之间的关系和文件名称，mega在数据库中单独维护了当前版本的目录信息

<!-- ### b-link file*
b-link 文件的作用是存储文件索引信息，用于替换git中的blob，该结构的设计参考了git-lfs的spec，如下:

  ```bash
  version https://mega.com/directory/spec/v1
  blob 3a739f77180d81aa45d9bd11eb6be7098bf1991f
  ```
它包含以下记录：
- version：代表结构的版本信息
- blob：指向真实的blob的hash值 -->

### Import directory
- 导入目录的主要作用是将原始的git仓库同步到mega目录中，在导入目录中的项目维持**只读**的状态，并保持项目的原始commit信息
- 往导入目录中推送的项目可以允许有多个commit
- 导入目录的项目可以快速的转化到mega目录
- 导入目录可以在配置文件中进行配置
- 一旦某个目录被初始化为import目录，就不再能修改回普通目录

## 2. Database Design

### Table Overall

| Table Name     | Description                                                                                             | MR Push  | Pull     | Push Repo | Pull Repo |
| -------------- | ------------------------------------------------------------------------------------------------------- | -------- | -------- | --------- | --------- |
| mega_directory | Mainain the latest directory stucture in monorepo.                                                      | &#10003; | &#10003; |           |           |
| mega_commit    | Store all commit objects related with mega directory, have mr status                                    | &#10003; |          |           |           |
| mega_tree      | Store all tree objects related with mega directory, together with mega_commit to find history directory | &#10003; |          |           |           |
| mega_blob      | Store all blob objects under mega directory.                                                            | &#10003; | &#10003; |           |           |
| merge_request  | Merge request related to mega commits.                                                                  | &#10003; |          |           |           |
| import_repo    | Maintain Relations between impoprt_repo and repo_path.                                                  |          |          | &#10003;  | &#10003;  |
| import_refs    | Obtains the latest commit_id through repo_id and ref_name, while also storing the tag.                  |          |          | &#10003;  | &#10003;  |
| impoprt_commit | Store all parsed commit objects related with repo.                                                      |          |          | &#10003;  | &#10003;  |
| import_tree    | Store all parsed tree objects related with repo.                                                        |          |          | &#10003;  | &#10003;  |
| import_blob    | Store all parsed blob objects related with repo.                                                        |          |          | &#10003;  | &#10003;  |
| raw_objects    | Store all raw objects with both repo and mega directory.                                                | &#10003; | &#10003; | &#10003;  | &#10003;  |
| pull_request   | Pull request sync from GitHub.                                                                          |          |          |           |           |
| issue          | Issues sync from GitHub.                                                                                |          |          |           |           |
| lfs_objects    | Store objects related to LFS protocol.                                                                  |          |          |           |           |
| lfs_locks      | Store locks for lfs files.                                                                              |          |          |           |           |

#### mega_directory

| Column     | Type        | Constraints |
| ---------- | ----------- | ----------- |
| id         | BIGINT      | PRIMARY KEY |
| path       | TEXT        | NOT NULL    |
| import_dir | BOOLEAN     | NOT NULL    |
| tree_id    | VARCHAR(40) | NOT NULL    |
| sub_trees  | TEXT[]      |             |
| commit_id  | VARCHAR(40) | NOT NULL    |
| size       | INT         | NOT NULL    |
| created_at | TIMESTAMP   | NOT NULL    |

#### mega_commit

| Column     | Type        | Constraints |
| ---------- | ----------- | ----------- |
| id         | BIGINT      | PRIMARY KEY |
| git_id     | VARCHAR(40) | NOT NULL    |
| tree       | VARCHAR(40) | NOT NULL    |
| pid        | TEXT[]      |             |
| author     | TEXT        |             |
| committer  | TEXT        |             |
| content    | TEXT        |             |
| mr_id      | VARCHAR(20) |             |
| status     | VARCHAR(20) | NOT NULL    |
| size       | INT         | NOT NULL    |
| full_path  | TEXT        | NOT NULL    |
| created_at | TIMESTAMP   | NOT NULL    |
| updated_at | TIMESTAMP   | NOT NULL    |


#### mega_tree

| Column     | Type        | Constraints |
| ---------- | ----------- | ----------- |
| id         | BIGINT      | PRIMARY KEY |
| git_id     | VARCHAR(40) | NOT NULL    |
| sub_trees  | TEXT[]      |             |
| import_dir | BOOLEAN     | NOT NULL    |
| mr_id      | VARCHAR(20) |             |
| status     | VARCHAR(20) | NOT NULL    |
| size       | INT         | NOT NULL    |
| full_path  | TEXT        | NOT NULL    |
| created_at | TIMESTAMP   | NOT NULL    |
| updated_at | TIMESTAMP   | NOT NULL    |

#### mega_blob

| Column     | Type        | Constraints |
| ---------- | ----------- | ----------- |
| id         | BIGINT      | PRIMARY KEY |
| git_id     | VARCHAR(40) | NOT NULL    |
| commit_id  | VARCHAR(40) | NOT NULL    |
| mr_id      | VARCHAR(20) |             |
| status     | VARCHAR(20) | NOT NULL    |
| size       | INT         | NOT NULL    |
| full_path  | TEXT        | NOT NULL    |
| created_at | TIMESTAMP   | NOT NULL    |
| updated_at | TIMESTAMP   | NOT NULL    |


#### merge_request

| Column     | Type         | Constraints |
| ---------- | ------------ | ----------- |
| id         | BIGINT       | PRIMARY KEY |
| mr_id      | BIGINT       | NOT NULL    |
| mr_msg     | VARCHAR(255) | NOT NULL    |
| commit_id  | VARCHAR(40)  | NOT NULL    |
| mr_date    | TIMESTAMP    | NOT NULL    |
| status     | VARCHAR(20)  | NOT NULL    |
| created_at | TIMESTAMP    | NOT NULL    |
| updated_at | TIMESTAMP    | NOT NULL    |

#### import_refs

| Column     | Type        | Constraints |
| ---------- | ----------- | ----------- |
| id         | BIGINT      | PRIMARY KEY |
| repo_id    | BIGINT      | NOT NULL    |
| ref_name   | TEXT        | NOT NULL    |
| ref_git_id | VARCHAR(40) | NOT NULL    |
| created_at | TIMESTAMP   | NOT NULL    |
| updated_at | TIMESTAMP   | NOT NULL    |


#### import_repo

| Column     | Type      | Constraints |
| ---------- | --------- | ----------- |
| id         | BIGINT    | PRIMARY KEY |
| repo_path  | TEXT      | NOT NULL    |
| created_at | TIMESTAMP | NOT NULL    |
| updated_at | TIMESTAMP | NOT NULL    |

#### import_commit

| Column     | Type        | Constraints |
| ---------- | ----------- | ----------- |
| id         | BIGINT      | PRIMARY KEY |
| repo_id    | BIGINT      | NOT NULL    |
| git_id     | VARCHAR(40) | NOT NULL    |
| tree       | VARCHAR(40) | NOT NULL    |
| pid        | TEXT[]      |             |
| author     | TEXT        |             |
| committer  | TEXT        |             |
| content    | TEXT        |             |
| size       | INT         | NOT NULL    |
| full_path  | TEXT        | NOT NULL    |
| created_at | TIMESTAMP   | NOT NULL    |

#### import_tree

| Column     | Type         | Constraints |
| ---------- | ------------ | ----------- |
| id         | BIGINT       | PRIMARY KEY |
| repo_id    | BIGINT       | NOT NULL    |
| git_id     | VARCHAR(40)  | NOT NULL    |
| sub_trees  | TEXT[]       |             |
| name       | VARCHAR(128) |             |
| size       | INT          | NOT NULL    |
| full_path  | TEXT         | NOT NULL    |
| commit_id  | VARCHAR(40)  | NOT NULL    |
| created_at | TIMESTAMP    | NOT NULL    |

#### import_blob

| Column     | Type         | Constraints |
| ---------- | ------------ | ----------- |
| id         | BIGINT       | PRIMARY KEY |
| repo_id    | BIGINT       | NOT NULL    |
| git_id     | VARCHAR(40)  | NOT NULL    |
| name       | VARCHAR(128) |             |
| size       | INT          | NOT NULL    |
| full_path  | TEXT         | NOT NULL    |
| commit_id  | VARCHAR(40)  | NOT NULL    |
| created_at | TIMESTAMP    | NOT NULL    |

#### raw_objects

| Column             | Type        | Constraints |
| ------------------ | ----------- | ----------- |
| id                 | BIGINT      | PRIMARY KEY |
| git_id             | VARCHAR(40) | NOT NULL    |
| object_type        | VARCHAR(20) | NOT NULL    |
| storage_type       | VARCHAR(20) | NOT NULL    |
| data               | BYTEA       |             |
| local_storage_path | TEXT        |             |
| remote_url         | TEXT        |             |


#### pull_request

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


#### issue

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

| Column  | Type        | Constraints |
| ------- | ----------- | ----------- |
| oid     | VARCHAR(64) | PRIMARY KEY |
| size    | BIGINT      |             |
| repo_id | BIGINT      | NOT NULL    |
| exist   | BOOLEAN     |             |


## 3. Sql execution for each process.


#### Use mega init command to initialize mega directory: 

- Init commit points to tree:
    ```sql
    insert into mega_commit values (...);
    ```
- Build directory and tree objs:
    ```sql
    insert into mega_directory values ('/root', false, ...);
    insert into mega_directory values ('/root/projects', false, ...);
    insert into mega_directory values ('/root/import', true, ...);
    insert into mega_directory values ('/root/projects/rust', false, ...);
    insert into mega_tree values (...);
    ```
- Generate ReadMe.md file and insert to raw_objects:
    ```sql
    insert into mega_blob values (id, git_id, 1024, 0, 'Merged');
    insert into raw_objects values (...);
    ```


#### Clone mega directory

- Check path is import directory
    ```sql
    select * from mega_directory where path = '/path/to/directory';
    ```
- If it's a mega directory

  - Check clone object size exceed the threshold:
    ```sql
    <!-- get all files under path -->
    select * from mega_directory where git_id in (...);
    <!-- calculate objects size with all blob ids -->
    select * from mega_blob where git_id in (...);
    ```
  - Construct new commit
    ```sql
    <!-- get related commit -->
    select * from mega_commit where git_id = commit_id
    ```
  - Pack file with new commit and raw tree and objects;
    ```sql
    <!-- get related trees and objects -->
    select * from raw_objects where git_id in (...);
    ```

- Or a import directory(see clone a repo)

#### Push back mega directory
- Parse packfile get trees and objs
  ```sql
  select * from mega_directory where path = '/path/to/directory';
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
  update mega_directory set (commit_id, sub_trees) where path = ?;
  ```

#### Clone repo 
  - Find related objects
    ```sql
    select * from import_repo where repo_path = '/path/to/repo'
    select git_id from import_commit where repo_id = ?;
    select git_id from import_tree where repo_id = ?;
    select git_id from import_blob where repo_id = ?;
    ```
  - Find raw objects by id
  ```sql
  select * from raw_objects where repo_id =? and git_id in (...);
  ```  
  - Pack file with raw_objetcs


#### Push back repo
- Check server refs by path
  ```sql
  select * from import_repo where repo_path = '/path/to/repo'
  select * from import_refs where repo_id = ...;
  ```
- Parse pack file and save objects
  ```sql
  insert into raw_objectss values(...);
  <!-- convert raw_obj to objects -->
  ```
- If under import directory
  ```sql
  insert into commit values (c1),(c2),(c3);
  insert into tree values (T1)...(T4);
  insert into blob values (B1)...(B5);
  ```
- Update refs
  ```sql
  update import_refs set ref_git_id = ? where repo_id =?;
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