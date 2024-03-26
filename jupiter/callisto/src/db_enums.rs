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

impl ToString for StorageType {
    fn to_string(&self) -> String {
        match self {
            StorageType::Database => String::from("database"),
            StorageType::LocalFs => String::from("local_fs"),
            StorageType::RemoteUrl => String::from("remote_url"),
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
    #[sea_orm(string_value = "reciew")]
    Review,
    #[sea_orm(string_value = "approve")]
    Approve,
    #[sea_orm(string_value = "merge_queue")]
    MergeQueue,
}
