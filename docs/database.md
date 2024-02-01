

## 1.Basic Design of Mega 

![Mega Directory Design](images/mega-tree-directory.png)

将Mega的存储结构拆分为两部分进行设计
- Part1：树形目录结构（接下来称为Mega Directory），其为一个git仓库，维护Projects，Import等Mega 目录和其下的README文件，对于需要存储的git仓库，将其存储为一个blob文件（图中R1，R2文件，接下来称为b-link文件），具体的内容可以参考lfs的spec，如:

    ```bash
    version https://mega.com/directory/spec/v1
    path /path/to/your/repo
    ```
- Part 2：Mega托管的git仓库本身，该部分则是通过将Packfile解析后的内容存储到数据库相应的表来进行维护

### Clone的大致流程

- 当进行clone时，首先会遍历最新提交的tree，并判断每个blob文件是否是b-link文件，如果是则获取指向的git仓库的大小，同时如果本次clone下所有b-link文件指向的仓库操作一个阈值，那么直接结束clone并返回错误.
- 未超出大小的前提下，则将所有b-link指向git仓库的最新目录树替换b-link文件，并**递归重新计算**Mega Directory中涉及到的tree节点和commit的hash，这样才能把完整的目录发送给client.
- 因为b-link只是记录了一个项目路径，并且一旦创建后文件本身不会变化，所以托管的git仓库的commit不会影响Mega Directory
- 如果对Mega Directory下的目录和文件进行修改，则需要产生新的commit，用于进行历史版本的回溯
- 值得注意的是Mega Directory 回溯，不会导致b-link对应的git仓库进行回溯，但是如果Mega Directory 在回溯中失去了b-link文件，则也会失去对应的git仓库


pack <==> raw obj <==> plain obj


## 2. Database Design

### Table Overall

| Table Name     | Description                                                                              |
| -------------- | ---------------------------------------------------------------------------------------- |
| refs           | Obtains the latest commit_id through repo_path and ref_name, while also storing the tag. |
| mega_directory | Mainain the latest tree stucture, and point to tree objs in dir_tree table.              |
| dir_commit     | Stored all commit objects related with mega directory.                                   |
| dir_tree       | Stored all tree objects related with mega directory.                                     |
| commit         | Stored all commit objects related with repo.                                             |
| tree           | Stored all tree objects related with repo.                                               |
| raw_objects    | Stored all raw objects both with repo and mega directory.                                |
| merge_request  | Merge request related to some commit.                                                    |
| pull_request   | Pull request synced from GitHub.                                                         |
| issue          | Issues synced from GitHub.                                                               |
| lfs_objects    | Stored objects related to LFS protocol.                                                  |
| lfs_locks      | Stored locks for lfs files.                                                              |

#### mega_directory

| Column     | Type        | Constraints |
| ---------- | ----------- | ----------- |
| id         | BIGINT      | PRIMARY KEY |
| full_path  | TEXT        | NOT NULL    |
| tree_id    | VARCHAR(40) | NOT NULL    |
| created_at | TIMESTAMP   | NOT NULL    |
| updated_at | TIMESTAMP   | NOT NULL    |

#### refs

| Column     | Type        | Constraints |
| ---------- | ----------- | ----------- |
| id         | BIGINT      | PRIMARY KEY |
| repo_path  | TEXT        | NOT NULL    |
| ref_name   | TEXT        | NOT NULL    |
| ref_git_id | VARCHAR(40) | NOT NULL    |
| created_at | TIMESTAMP   | NOT NULL    |
| updated_at | TIMESTAMP   | NOT NULL    |

#### dir_commit

| Column    | Type        | Constraints |
| --------- | ----------- | ----------- |
| id        | BIGINT      | PRIMARY KEY |
| git_id    | VARCHAR(40) | NOT NULL    |
| tree      | VARCHAR(40) | NOT NULL    |
| pid       | TEXT[]      |             |
| repo_path | TEXT        | NOT NULL    |
| author    | TEXT        |             |
| committer | TEXT        |             |
| content   | TEXT        |             |


#### dir_tree

| Column      | Type         | Constraints |
| ----------- | ------------ | ----------- |
| id          | BIGINT       | PRIMARY KEY |
| git_id      | VARCHAR(40)  | NOT NULL    |
| last_commit | VARCHAR(40)  | NOT NULL    |
| name        | VARCHAR(128) |             |
| sub_trees   | TEXT[]       |             |
| size        | INT          | NOT NULL    |
| repo_path   | TEXT         | NOT NULL    |
| full_path   | TEXT         | NOT NULL    |


#### merge_request


| Column     | Type         | Constraints |
| ---------- | ------------ | ----------- |
| id         | BIGINT       | PRIMARY KEY |
| mr_id      | BIGINT       | NOT NULL    |
| mr_msg     | VARCHAR(255) | NOT NULL    |
| commit_id  | VARCHAR(40)  | NOT NULL    |
| mr_date    | TIMESTAMP    | NOT NULL    |
| created_at | TIMESTAMP    | NOT NULL    |
| updated_at | TIMESTAMP    | NOT NULL    |


#### raw_objects


| Column        | Type        | Constraints     |
| ------------- | ----------- | --------------- |
| id            | BIGINT      | PRIMARY KEY     |
| git_id        | VARCHAR(40) | NOT NULL        |
| object_type   | VARCHAR(16) | NOT NULL        |
| storage_type  | VARCHAR(20) | NOT NULL        |
| data          | BYTEA       |                 |
| path          | TEXT        |                 |
| url           | TEXT        |                 |
| uniq_o_git_id | CONSTRAINT  | UNIQUE (git_id) |


#### commit

| Column     | Type        | Constraints |
| ---------- | ----------- | ----------- |
| id         | BIGINT      | PRIMARY KEY |
| git_id     | VARCHAR(40) | NOT NULL    |
| tree       | VARCHAR(40) | NOT NULL    |
| pid        | TEXT[]      |             |
| repo_path  | TEXT        | NOT NULL    |
| author     | TEXT        |             |
| committer  | TEXT        |             |
| content    | TEXT        |             |
| mr_id      | VARCHAR(20) |             |
| status     | VARCHAR(20) | NOT NULL    |
| created_at | TIMESTAMP   | NOT NULL    |
| updated_at | TIMESTAMP   | NOT NULL    |


#### tree

| Column      | Type         | Constraints |
| ----------- | ------------ | ----------- |
| id          | BIGINT       | PRIMARY KEY |
| git_id      | VARCHAR(40)  | NOT NULL    |
| last_commit | VARCHAR(40)  | NOT NULL    |
| name        | VARCHAR(128) |             |
| sub_trees   | TEXT[]       |             |
| size        | INT          | NOT NULL    |
| repo_path   | TEXT         | NOT NULL    |
| full_path   | TEXT         | NOT NULL    |
| mr_id       | VARCHAR(20)  |             |
| status      | VARCHAR(20)  | NOT NULL    |
| created_at  | TIMESTAMP    | NOT NULL    |
| updated_at  | TIMESTAMP    | NOT NULL    |


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
| repo_path        | TEXT         | NOT NULL     |
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
| repo_path   | TEXT         | NOT NULL     |
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


## 3. 流程对应的sql语句



#### Use mega init command to initialize mega directory: 

- Generate ReadMe.md file and insert to raw_objects:
    ```sql
    insert into raw_objects values (...);
    ```
- Build directory and tree objs:
    ```sql
    insert into mega_directory values ('/root', ...);
    insert into mega_directory values ('/root/projects', ...);
    insert into mega_directory values ('/root/import', ...);
    insert into mega_directory values ('/root/projects/rust', ...);
    insert into dir_tree values (...);
    ```
- Init commit points to tree and update refs:
    ```sql
    insert into dir_commit values (...);
    insert into refs value ('/root', commit_id);
    ```


#### Clone mega directory

- check path is a repo or a mega directory
    ```sql
    select * from mega_directory where path = '/path/by/client';
    ```
- If it's a mega directory

  - Check clone limit:
    ```sql
    <!-- got related commit -->
    select commit_id from refs where repo_path = "/root" ;
    <!-- calculate objects size -->
    select * from dir_tree where tree_id = '...';
    select * from raw_objects where git_id in (...);
    ```
  - Parse file and check if it's a b-link file
  - Replace b-link with repo(same as clone a repo)
  - construct new tree and commit
  - pack file with new commit and tree


- Or a repo(see clone a repo)

#### Push back mega directory
- clone mega directory and then update readme or directory
- TODO

#### Init repo under mega directory(no need MR)

TODO

#### Clone repo 
  - find related objects
    ```sql
    select * from refs where repo_path = '/path/by/client';
    select * from commit where commit_id = ...;
    select * from tree where git_id = ...;
    select * from raw_objects where git_id in (...);
    ```
  - pack file with raw_objetcs


#### Push back repo and open merge request

TODO



## 4. clone时遵守的规则（TODO）

### ✅ git clone root：
- 1个commit，只包含一级目录Projects，Import 和ReadME
- 用于改readme等文件，添加和修改其他文件会报错
- 需要记录目录的历史版本
- 判断contains repo

### ✅ git clone projects：
- 1个commit C-Project，包含底下的所有项目，根据目录计算出projetcs🌲
- 需要给定阈值来限制clone的大小，超出则通过api来进行修改

### ✅ git clone projects/repo：
- 1个commit，将C3的parent改为空

### ✅ git clone projects/repo/T3 ： 
- 1个commit，将C3的parent改为空，并指向T3

### ❌ git clone import：
- 不允许，因为不能把多个项目合并成一个项目

### ✅ git clone import/repo：
- 包含所有历史提交的标准clone

### ❌ git clone import/repo/T3：
- 不允许子目录clone

## 4. Prerequisites

- You need to execute SQL files in a specific order to init the database.

    For example using `PostgreSQL`, execute the files under `sql\postgres`:

        pg_20230803__init.sql

    or if your are using `Mysql`, execute scripts:

        mysql_20230523__init.sql



- Generating entities: 
Entities can be generated from the database table structure with the following command

`sea-orm-cli generate entity -u "mysql://${DB_USERNAME}:${DB_SECRET}@${DB_HOST}/mega"  -o database/entity/src` 