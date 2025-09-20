//! Implements From trait for converting mercury models to callisto models

impl From<mercury::internal::model::sea_models::git_commit::Model> for crate::git_commit::Model {
    fn from(model: mercury::internal::model::sea_models::git_commit::Model) -> Self {
        Self {
            id: model.id,
            repo_id: model.repo_id as i64, // Convert i32 to i64
            commit_id: model.commit_id,
            tree: model.tree,
            parents_id: serde_json::from_str(&model.parents_id).unwrap_or(serde_json::Value::Array(vec![])),
            author: model.author,
            committer: model.committer,
            content: model.content,
            created_at: model.created_at, // Keep as NaiveDateTime to match callisto
        }
    }
}

impl From<mercury::internal::model::sea_models::git_commit::ActiveModel> for crate::git_commit::ActiveModel {
    fn from(model: mercury::internal::model::sea_models::git_commit::ActiveModel) -> Self {
        Self {
            id: model.id,
            repo_id: match model.repo_id {
                sea_orm::ActiveValue::Set(val) => sea_orm::ActiveValue::Set(val as i64), // Convert i32 to i64
                sea_orm::ActiveValue::Unchanged(val) => sea_orm::ActiveValue::Unchanged(val as i64), // Convert i32 to i64
                sea_orm::ActiveValue::NotSet => sea_orm::ActiveValue::NotSet,
            },
            commit_id: model.commit_id,
            tree: model.tree,
            parents_id: match model.parents_id {
                sea_orm::ActiveValue::Set(val) => sea_orm::ActiveValue::Set(
                    serde_json::from_str(&val).unwrap_or(serde_json::Value::Array(vec![]))
                ),
                sea_orm::ActiveValue::Unchanged(val) => sea_orm::ActiveValue::Unchanged(
                    serde_json::from_str(&val).unwrap_or(serde_json::Value::Array(vec![]))
                ),
                sea_orm::ActiveValue::NotSet => sea_orm::ActiveValue::NotSet,
            },
            author: model.author,
            committer: model.committer,
            content: model.content,
            created_at: match model.created_at {
                sea_orm::ActiveValue::Set(val) => sea_orm::ActiveValue::Set(val),
                sea_orm::ActiveValue::Unchanged(val) => sea_orm::ActiveValue::Unchanged(val),
                sea_orm::ActiveValue::NotSet => sea_orm::ActiveValue::NotSet,
            },
        }
    }
}

impl From<mercury::internal::model::sea_models::mega_commit::Model> for crate::mega_commit::Model {
    fn from(model: mercury::internal::model::sea_models::mega_commit::Model) -> Self {
        Self {
            id: model.id,
            commit_id: model.commit_id,
            tree: model.tree,
            parents_id: serde_json::from_str(&model.parents_id).unwrap_or(serde_json::Value::Array(vec![])),
            author: model.author,
            committer: model.committer,
            content: model.content,
            created_at: model.created_at, // Keep as NaiveDateTime to match callisto
        }
    }
}

impl From<mercury::internal::model::sea_models::mega_commit::ActiveModel> for crate::mega_commit::ActiveModel {
    fn from(model: mercury::internal::model::sea_models::mega_commit::ActiveModel) -> Self {
        Self {
            id: model.id,
            commit_id: model.commit_id,
            tree: model.tree,
            parents_id: match model.parents_id {
                sea_orm::ActiveValue::Set(val) => sea_orm::ActiveValue::Set(
                    serde_json::from_str(&val).unwrap_or(serde_json::Value::Array(vec![]))
                ),
                sea_orm::ActiveValue::Unchanged(val) => sea_orm::ActiveValue::Unchanged(
                    serde_json::from_str(&val).unwrap_or(serde_json::Value::Array(vec![]))
                ),
                sea_orm::ActiveValue::NotSet => sea_orm::ActiveValue::NotSet,
            },
            author: model.author,
            committer: model.committer,
            content: model.content,
            created_at: match model.created_at {
                sea_orm::ActiveValue::Set(val) => sea_orm::ActiveValue::Set(val),
                sea_orm::ActiveValue::Unchanged(val) => sea_orm::ActiveValue::Unchanged(val),
                sea_orm::ActiveValue::NotSet => sea_orm::ActiveValue::NotSet,
            },
        }
    }
}

// -------------------------
// mega_tree adapters
// -------------------------
impl From<mercury::internal::model::sea_models::mega_tree::Model> for crate::mega_tree::Model {
    fn from(model: mercury::internal::model::sea_models::mega_tree::Model) -> Self {
        Self {
            id: model.id,
            tree_id: model.tree_id,
            sub_trees: model.sub_trees,
            size: model.size,
            commit_id: model.commit_id,
            created_at: model.created_at,
        }
    }
}

impl From<mercury::internal::model::sea_models::mega_tree::ActiveModel> for crate::mega_tree::ActiveModel {
    fn from(model: mercury::internal::model::sea_models::mega_tree::ActiveModel) -> Self {
        Self {
            id: model.id,
            tree_id: model.tree_id,
            sub_trees: model.sub_trees,
            size: model.size,
            commit_id: model.commit_id,
            created_at: model.created_at,
        }
    }
}

// -------------------------
// mega_blob adapters
// -------------------------
impl From<mercury::internal::model::sea_models::mega_blob::Model> for crate::mega_blob::Model {
    fn from(model: mercury::internal::model::sea_models::mega_blob::Model) -> Self {
        Self {
            id: model.id,
            blob_id: model.blob_id,
            commit_id: model.commit_id,
            name: model.name,
            size: model.size,
            created_at: model.created_at,
        }
    }
}

impl From<mercury::internal::model::sea_models::mega_blob::ActiveModel> for crate::mega_blob::ActiveModel {
    fn from(model: mercury::internal::model::sea_models::mega_blob::ActiveModel) -> Self {
        Self { id: model.id, blob_id: model.blob_id, commit_id: model.commit_id, name: model.name, size: model.size, created_at: model.created_at }
    }
}

// -------------------------
// raw_blob adapters (with StorageTypeEnum mapping)
// -------------------------
impl From<mercury::internal::model::sea_models::raw_blob::Model> for crate::raw_blob::Model {
    fn from(model: mercury::internal::model::sea_models::raw_blob::Model) -> Self {
        let storage_type = match model.storage_type.as_str() {
            "database" | "Database" => crate::sea_orm_active_enums::StorageTypeEnum::Database,
            "local_fs" | "LocalFs" | "fs" => crate::sea_orm_active_enums::StorageTypeEnum::LocalFs,
            "aws_s3" | "s3" | "S3" => crate::sea_orm_active_enums::StorageTypeEnum::AwsS3,
            _ => crate::sea_orm_active_enums::StorageTypeEnum::Database,
        };
        Self {
            id: model.id,
            sha1: model.sha1,
            content: model.content,
            file_type: model.file_type,
            storage_type,
            data: model.data,
            local_path: model.local_path,
            remote_url: model.remote_url,
            created_at: model.created_at,
        }
    }
}

impl From<mercury::internal::model::sea_models::raw_blob::ActiveModel> for crate::raw_blob::ActiveModel {
    fn from(model: mercury::internal::model::sea_models::raw_blob::ActiveModel) -> Self {
        let map_storage = |s: String| match s.as_str() {
            "database" | "Database" => crate::sea_orm_active_enums::StorageTypeEnum::Database,
            "local_fs" | "LocalFs" | "fs" => crate::sea_orm_active_enums::StorageTypeEnum::LocalFs,
            "aws_s3" | "s3" | "S3" => crate::sea_orm_active_enums::StorageTypeEnum::AwsS3,
            _ => crate::sea_orm_active_enums::StorageTypeEnum::Database,
        };
        Self {
            id: model.id,
            sha1: model.sha1,
            content: model.content,
            file_type: model.file_type,
            storage_type: match model.storage_type {
                sea_orm::ActiveValue::Set(v) => sea_orm::ActiveValue::Set(map_storage(v)),
                sea_orm::ActiveValue::Unchanged(v) => sea_orm::ActiveValue::Unchanged(map_storage(v)),
                sea_orm::ActiveValue::NotSet => sea_orm::ActiveValue::NotSet,
            },
            data: model.data,
            local_path: model.local_path,
            remote_url: model.remote_url,
            created_at: model.created_at,
        }
    }
}

// -------------------------
// mega_tag adapters
// -------------------------
impl From<mercury::internal::model::sea_models::mega_tag::Model> for crate::mega_tag::Model {
    fn from(model: mercury::internal::model::sea_models::mega_tag::Model) -> Self {
        Self {
            id: model.id,
            tag_id: model.tag_id,
            object_id: model.object_id,
            object_type: model.object_type,
            tag_name: model.tag_name,
            tagger: model.tagger,
            message: model.message,
            created_at: model.created_at,
        }
    }
}

impl From<mercury::internal::model::sea_models::mega_tag::ActiveModel> for crate::mega_tag::ActiveModel {
    fn from(model: mercury::internal::model::sea_models::mega_tag::ActiveModel) -> Self {
        Self {
            id: model.id,
            tag_id: model.tag_id,
            object_id: model.object_id,
            object_type: model.object_type,
            tag_name: model.tag_name,
            tagger: model.tagger,
            message: model.message,
            created_at: model.created_at,
        }
    }
}

// -------------------------
// git_tree adapters
// -------------------------
impl From<mercury::internal::model::sea_models::git_tree::Model> for crate::git_tree::Model {
    fn from(model: mercury::internal::model::sea_models::git_tree::Model) -> Self {
        Self {
            id: model.id,
            repo_id: model.repo_id as i64,
            tree_id: model.tree_id,
            sub_trees: model.sub_trees,
            size: model.size,
            created_at: model.created_at,
        }
    }
}

impl From<mercury::internal::model::sea_models::git_tree::ActiveModel> for crate::git_tree::ActiveModel {
    fn from(model: mercury::internal::model::sea_models::git_tree::ActiveModel) -> Self {
        Self {
            id: model.id,
            repo_id: match model.repo_id {
                sea_orm::ActiveValue::Set(v) => sea_orm::ActiveValue::Set(v as i64),
                sea_orm::ActiveValue::Unchanged(v) => sea_orm::ActiveValue::Unchanged(v as i64),
                sea_orm::ActiveValue::NotSet => sea_orm::ActiveValue::NotSet,
            },
            tree_id: model.tree_id,
            sub_trees: model.sub_trees,
            size: model.size,
            created_at: model.created_at,
        }
    }
}

// -------------------------
// git_blob adapters
// -------------------------
impl From<mercury::internal::model::sea_models::git_blob::Model> for crate::git_blob::Model {
    fn from(model: mercury::internal::model::sea_models::git_blob::Model) -> Self {
        Self {
            id: model.id,
            repo_id: model.repo_id as i64,
            blob_id: model.blob_id,
            name: model.name,
            size: model.size,
            created_at: model.created_at,
        }
    }
}

impl From<mercury::internal::model::sea_models::git_blob::ActiveModel> for crate::git_blob::ActiveModel {
    fn from(model: mercury::internal::model::sea_models::git_blob::ActiveModel) -> Self {
        Self {
            id: model.id,
            repo_id: match model.repo_id {
                sea_orm::ActiveValue::Set(v) => sea_orm::ActiveValue::Set(v as i64),
                sea_orm::ActiveValue::Unchanged(v) => sea_orm::ActiveValue::Unchanged(v as i64),
                sea_orm::ActiveValue::NotSet => sea_orm::ActiveValue::NotSet,
            },
            blob_id: model.blob_id,
            name: model.name,
            size: model.size,
            created_at: model.created_at,
        }
    }
}

// -------------------------
// git_tag adapters
// -------------------------
impl From<mercury::internal::model::sea_models::git_tag::Model> for crate::git_tag::Model {
    fn from(model: mercury::internal::model::sea_models::git_tag::Model) -> Self {
        Self {
            id: model.id,
            repo_id: model.repo_id as i64,
            tag_id: model.tag_id,
            object_id: model.object_id,
            object_type: model.object_type,
            tag_name: model.tag_name,
            tagger: model.tagger,
            message: model.message,
            created_at: model.created_at,
        }
    }
}

impl From<mercury::internal::model::sea_models::git_tag::ActiveModel> for crate::git_tag::ActiveModel {
    fn from(model: mercury::internal::model::sea_models::git_tag::ActiveModel) -> Self {
        Self {
            id: model.id,
            repo_id: match model.repo_id {
                sea_orm::ActiveValue::Set(v) => sea_orm::ActiveValue::Set(v as i64),
                sea_orm::ActiveValue::Unchanged(v) => sea_orm::ActiveValue::Unchanged(v as i64),
                sea_orm::ActiveValue::NotSet => sea_orm::ActiveValue::NotSet,
            },
            tag_id: model.tag_id,
            object_id: model.object_id,
            object_type: model.object_type,
            tag_name: model.tag_name,
            tagger: model.tagger,
            message: model.message,
            created_at: model.created_at,
        }
    }
}