use core::fmt;
use std::fmt::Display;

use sea_orm::prelude::StringLen;
use sea_orm::{DeriveActiveEnum, EnumIter};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "snake_case"
)]
pub enum StorageType {
    Database,
    LocalFs,
    RemoteUrl,
}

impl fmt::Display for StorageType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StorageType::Database => write!(f, "database"),
            StorageType::LocalFs => write!(f, "local_fs"),
            StorageType::RemoteUrl => write!(f, "remote_url"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Copy)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "snake_case"
)]
pub enum MergeStatus {
    Open,
    Merged,
    Closed,
}

impl Display for MergeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            MergeStatus::Open => "open",
            MergeStatus::Merged => "merged",
            MergeStatus::Closed => "closed",
        };
        write!(f, "{}", s)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Deserialize, Serialize)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "snake_case"
)]
pub enum RefType {
    Branch,
    Tag,
}

#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "snake_case"
)]
pub enum ConvType {
    Comment,
    Deploy,
    Commit,
    ForcePush,
    Edit,
    Review,
    Approve,
    MergeQueue,
    Merged,
    Closed,
    Reopen,
}

impl Display for ConvType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ConvType::Comment => "Comment",
            ConvType::Deploy => "Deploy",
            ConvType::Commit => "Commit",
            ConvType::ForcePush => "ForcePush",
            ConvType::Edit => "Edit",
            ConvType::Review => "Review",
            ConvType::Approve => "Approve",
            ConvType::MergeQueue => "MergeQueue",
            ConvType::Merged => "Merged",
            ConvType::Closed => "Closed",
            ConvType::Reopen => "Reopen",
        };
        write!(f, "{}", s)
    }
}
