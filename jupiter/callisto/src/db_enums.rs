use core::fmt;
use std::fmt::Display;

use sea_orm::{DeriveActiveEnum, EnumIter};

#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(Some(1))")]
pub enum StorageType {
    #[sea_orm(string_value = "database")]
    Database,
    #[sea_orm(string_value = "local_fs")]
    LocalFs,
    #[sea_orm(string_value = "remote_url")]
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
#[sea_orm(rs_type = "String", db_type = "String(Some(1))")]
pub enum MergeStatus {
    #[sea_orm(string_value = "open")]
    Open,
    #[sea_orm(string_value = "merged")]
    Merged,
    #[sea_orm(string_value = "closed")]
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

#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(Some(1))")]
pub enum RefType {
    #[sea_orm(string_value = "branch")]
    Branch,
    #[sea_orm(string_value = "tag")]
    Tag,
}


#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(Some(1))")]
pub enum ConvType {
    #[sea_orm(string_value = "comment")]
    Comment,
    #[sea_orm(string_value = "deploy")]
    Deploy,
    #[sea_orm(string_value = "commit")]
    Commit,
    #[sea_orm(string_value = "forse_push")]
    ForcePush,
    #[sea_orm(string_value = "edit")]
    Edit,   
    #[sea_orm(string_value = "review")]
    Review,
    #[sea_orm(string_value = "approve")]
    Approve,
    #[sea_orm(string_value = "merge_queue")]
    MergeQueue,
    #[sea_orm(string_value = "merged")]
    Merged,
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
        };
        write!(f, "{}", s)
    }
}
