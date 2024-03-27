use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "config_entry")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub entry_id: i32,
    pub section_id: i32,
    pub key: String,
    pub value: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::config_section::Entity",
        from = "Column::SectionId",
        to = "super::config_section::Column::SectionId"
    )]
    ConfigSection,
}

impl Related<super::config_section::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ConfigSection.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
