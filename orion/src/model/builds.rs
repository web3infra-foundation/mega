use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "builds")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub build_id: Uuid,
    pub output: String,
    pub exit_code: Option<i32>, // On Unix, return `None` if the process was terminated by a signal.
    pub start_at: DateTimeUtc,
    pub end_at: DateTimeUtc,
    pub repo_name: String,
    pub target: String, // build target, e.g. "//:main"
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub async fn get_by_build_id(build_id: Uuid, conn: DatabaseConnection) -> Option<Model> {
        Entity::find()
            .filter(Column::BuildId.eq(build_id))
            .one(&conn)
            .await
            .expect("Failed to get by `build_id`")
    }
}
