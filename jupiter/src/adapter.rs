use crate::callisto::git_commit;
use crate::callisto::git_tag;
use crate::callisto::git_tree;
use crate::callisto::mega_commit;
use crate::callisto::mega_tree;
use crate::callisto::raw_blob;
use mercury::hash::SHA1;
use mercury::internal::object::blob::Blob;
use mercury::internal::object::commit::Commit;
use mercury::internal::object::signature::Signature;
use mercury::internal::object::tag::Tag;
use mercury::internal::object::tree::Tree;
use mercury::internal::object::types::ObjectType;
use std::str::FromStr;

/// 将 callisto::raw_blob::Model 转换为 mercury::internal::object::blob::Blob
pub fn raw_blob_to_blob(model: raw_blob::Model) -> Blob {
    // 注意：这里我们不使用sha1，而是让Blob::from_content_bytes自己计算
    let data = model.data.unwrap_or_default();

    // 使用 Blob::from_content_bytes 方法
    Blob::from_content_bytes(data)
}

/// 将 mercury::internal::object::blob::Blob 转换为 callisto::raw_blob::Model
pub fn blob_to_raw_blob(blob: Blob) -> raw_blob::Model {
    use crate::callisto::sea_orm_active_enums::StorageTypeEnum;

    raw_blob::Model {
        id: 0,                        // 这个ID应该由数据库生成
        sha1: format!("{}", blob.id), // 使用Display trait
        content: None,
        file_type: None,
        storage_type: StorageTypeEnum::Database, // 使用Database作为默认存储类型
        data: Some(blob.data.clone()),
        local_path: None,
        remote_url: None,
        created_at: chrono::Utc::now().naive_utc(),
    }
}

/// 将 callisto::git_tree::Model 转换为 mercury::internal::object::tree::Tree
pub fn git_tree_to_tree(model: git_tree::Model) -> Tree {
    // 反序列化tree_items
    let (tree_items, _) =
        bincode::serde::decode_from_slice(&model.sub_trees, bincode::config::standard())
            .unwrap_or_else(|_| (Vec::new(), 0));

    // 使用from_tree_items构造Tree对象
    Tree::from_tree_items(tree_items).expect("Failed to create Tree")
}

/// 将 mercury::internal::object::tree::Tree 转换为 callisto::git_tree::Model
pub fn tree_to_git_tree(tree: Tree, repo_id: i64) -> git_tree::Model {
    // 序列化tree_items
    let sub_trees = bincode::serde::encode_to_vec(&tree.tree_items, bincode::config::standard())
        .expect("Failed to serialize tree items");

    let size = sub_trees.len() as i32;

    git_tree::Model {
        id: 0, // 这个ID应该由数据库生成
        repo_id,
        tree_id: format!("{}", tree.id), // 使用Display trait
        sub_trees,
        size,
        created_at: chrono::Utc::now().naive_utc(),
    }
}

/// 将 callisto::mega_tree::Model 转换为 mercury::internal::object::tree::Tree
pub fn mega_tree_to_tree(model: mega_tree::Model) -> Tree {
    // 反序列化tree_items
    let (tree_items, _) =
        bincode::serde::decode_from_slice(&model.sub_trees, bincode::config::standard())
            .unwrap_or_else(|_| (Vec::new(), 0));

    // 使用from_tree_items构造Tree对象
    Tree::from_tree_items(tree_items).expect("Failed to create Tree")
}

/// 将 mercury::internal::object::tree::Tree 转换为 callisto::mega_tree::Model
pub fn tree_to_mega_tree(tree: Tree, commit_id: &str) -> mega_tree::Model {
    // 序列化tree_items
    let sub_trees = bincode::serde::encode_to_vec(&tree.tree_items, bincode::config::standard())
        .expect("Failed to serialize tree items");

    let size = sub_trees.len() as i32;

    mega_tree::Model {
        id: 0,                           // 这个ID应该由数据库生成
        tree_id: format!("{}", tree.id), // 使用Display trait
        sub_trees,
        size,
        commit_id: commit_id.to_string(),
        created_at: chrono::Utc::now().naive_utc(),
    }
}

/// 将 callisto::git_commit::Model 转换为 mercury::internal::object::commit::Commit
pub fn git_commit_to_commit(model: git_commit::Model) -> Commit {
    // 解析父提交哈希列表
    let parent_commit_ids: Vec<SHA1> =
        serde_json::from_str::<Vec<String>>(model.parents_id.to_string().as_str())
            .unwrap_or_default()
            .iter()
            .map(|id| SHA1::from_str(id).unwrap())
            .collect();

    // 获取 author 和 committer 信息
    let author = model
        .author
        .map(|a| Signature::from_data(a.into_bytes()).unwrap())
        .unwrap_or_else(|| {
            Signature::from_data(
                format!(
                    "author unknown <unknown@example.com> {} +0000",
                    chrono::Utc::now().timestamp()
                )
                .into_bytes(),
            )
            .unwrap()
        });

    let committer = model
        .committer
        .map(|c| Signature::from_data(c.into_bytes()).unwrap())
        .unwrap_or_else(|| {
            Signature::from_data(
                format!(
                    "committer unknown <unknown@example.com> {} +0000",
                    chrono::Utc::now().timestamp()
                )
                .into_bytes(),
            )
            .unwrap()
        });

    // 创建 Commit 对象
    let message = model.content.unwrap_or_default();
    let tree_id = SHA1::from_str(&model.tree).unwrap();

    // 使用 Commit::new 方法创建 Commit
    // 由于 id 在 Commit::new 中会自动生成，忽略了model.commit_id
    let commit = Commit::new(author, committer, tree_id, parent_commit_ids, &message);

    // 由于Commit是non_exhaustive的，我们不能使用结构体更新语法
    // 如果id不匹配，可能需要在应用程序层面处理
    commit
}

/// 将 mercury::internal::object::commit::Commit 转换为 callisto::git_commit::Model
pub fn commit_to_git_commit(commit: Commit, repo_id: i64) -> git_commit::Model {
    // 将 parent_commit_ids 转换为 JSON 字符串
    let parents_id = serde_json::to_string(
        &commit
            .parent_commit_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<String>>(),
    )
    .unwrap();

    git_commit::Model {
        id: 0, // 由数据库生成
        repo_id,
        commit_id: commit.id.to_string(),
        tree: commit.tree_id.to_string(),
        parents_id: sea_orm::JsonValue::String(parents_id),
        author: Some(commit.author.to_string()),
        committer: Some(commit.committer.to_string()),
        content: Some(commit.message),
        created_at: chrono::Utc::now().naive_utc(),
    }
}

/// 将 callisto::mega_commit::Model 转换为 mercury::internal::object::commit::Commit
pub fn mega_commit_to_commit(model: mega_commit::Model) -> Commit {
    // 解析父提交哈希列表
    let parent_commit_ids: Vec<SHA1> =
        serde_json::from_str::<Vec<String>>(model.parents_id.to_string().as_str())
            .unwrap_or_default()
            .iter()
            .map(|id| SHA1::from_str(id).unwrap())
            .collect();

    // 获取 author 和 committer 信息
    let author = model
        .author
        .map(|a| Signature::from_data(a.into_bytes()).unwrap())
        .unwrap_or_else(|| {
            Signature::from_data(
                format!(
                    "author unknown <unknown@example.com> {} +0000",
                    chrono::Utc::now().timestamp()
                )
                .into_bytes(),
            )
            .unwrap()
        });

    let committer = model
        .committer
        .map(|c| Signature::from_data(c.into_bytes()).unwrap())
        .unwrap_or_else(|| {
            Signature::from_data(
                format!(
                    "committer unknown <unknown@example.com> {} +0000",
                    chrono::Utc::now().timestamp()
                )
                .into_bytes(),
            )
            .unwrap()
        });

    // 创建 Commit 对象
    let message = model.content.unwrap_or_default();
    let tree_id = SHA1::from_str(&model.tree).unwrap();

    // 使用 Commit::new 方法创建 Commit
    // 由于 id 在 Commit::new 中会自动生成，忽略了model.commit_id
    let commit = Commit::new(author, committer, tree_id, parent_commit_ids, &message);

    // 由于Commit是non_exhaustive的，我们不能使用结构体更新语法
    // 如果id不匹配，可能需要在应用程序层面处理
    commit
}

/// 将 mercury::internal::object::commit::Commit 转换为 callisto::mega_commit::Model
pub fn commit_to_mega_commit(commit: Commit) -> mega_commit::Model {
    // 将 parent_commit_ids 转换为 JSON 字符串
    let parents_id = serde_json::to_string(
        &commit
            .parent_commit_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<String>>(),
    )
    .unwrap();

    mega_commit::Model {
        id: 0, // 由数据库生成
        commit_id: commit.id.to_string(),
        tree: commit.tree_id.to_string(),
        parents_id: sea_orm::JsonValue::String(parents_id),
        author: Some(commit.author.to_string()),
        committer: Some(commit.committer.to_string()),
        content: Some(commit.message),
        created_at: chrono::Utc::now().naive_utc(),
    }
}

/// 将 callisto::git_tag::Model 转换为 mercury::internal::object::tag::Tag
pub fn git_tag_to_tag(model: git_tag::Model) -> Tag {
    // 解析对象类型
    let object_type = match model.object_type.as_str() {
        "commit" => ObjectType::Commit,
        "tree" => ObjectType::Tree,
        "blob" => ObjectType::Blob,
        "tag" => ObjectType::Tag,
        _ => ObjectType::Blob, // 默认为Blob类型
    };

    // 创建 tagger 签名
    let tagger = Signature::from_data(format!("tagger {}", model.tagger).into_bytes())
        .unwrap_or_else(|_| {
            Signature::from_data(
                format!(
                    "tagger unknown <unknown@example.com> {} +0000",
                    chrono::Utc::now().timestamp()
                )
                .into_bytes(),
            )
            .unwrap()
        });

    // 使用 Tag::new 方法构造 Tag
    let object_hash = SHA1::from_str(&model.object_id).unwrap();
    let tag_name = model.tag_name.clone();
    let message = model.message.clone();

    // 创建 Tag
    Tag::new(object_hash, object_type, tag_name, tagger, message)
}

/// 将 mercury::internal::object::tag::Tag 转换为 callisto::git_tag::Model
pub fn tag_to_git_tag(tag: Tag, repo_id: i64) -> git_tag::Model {
    git_tag::Model {
        id: 0, // 由数据库生成
        repo_id,
        tag_id: tag.id.to_string(),
        object_id: tag.object_hash.to_string(),
        object_type: tag.object_type.to_string(),
        tag_name: tag.tag_name,
        tagger: tag.tagger.to_string(),
        message: tag.message,
        created_at: chrono::Utc::now().naive_utc(),
    }
}
