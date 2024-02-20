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

#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(Some(1))")]
pub enum MergeStatus {
    #[sea_orm(string_value = "open")]
    Open,
    #[sea_orm(string_value = "merged")]
    Merged,
    #[sea_orm(string_value = "closed")]
    Closed,
}
