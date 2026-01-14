use chrono::Utc;
use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DbErr, RuntimeErr, sqlx};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, Copy, ToSchema,
)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum TargetState {
    #[sea_orm(string_value = "Pending")]
    Pending,
    #[sea_orm(string_value = "Building")]
    Building,
    #[sea_orm(string_value = "Completed")]
    Completed,
    #[sea_orm(string_value = "Failed")]
    Failed,
    #[sea_orm(string_value = "Interrupted")]
    Interrupted,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "targets")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub task_id: Uuid,
    pub target_path: String,
    pub state: TargetState,
    pub start_at: Option<DateTimeWithTimeZone>,
    pub end_at: Option<DateTimeWithTimeZone>,
    pub error_summary: Option<String>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::tasks::Entity",
        from = "Column::TaskId",
        to = "super::tasks::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Tasks,
    #[sea_orm(has_many = "super::builds::Entity")]
    Builds,
}

impl Related<super::tasks::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tasks.def()
    }
}

impl Related<super::builds::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Builds.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Create a new target ActiveModel ready for insertion
    pub fn create_target(task_id: Uuid, target_path: impl Into<String>) -> ActiveModel {
        let now = Utc::now().into();
        ActiveModel {
            id: Set(Uuid::now_v7()),
            task_id: Set(task_id),
            target_path: Set(target_path.into()),
            state: Set(TargetState::Pending),
            start_at: Set(None),
            end_at: Set(None),
            error_summary: Set(None),
            created_at: Set(now),
        }
    }

    /// Insert a target directly into the database
    pub async fn insert_target(
        task_id: Uuid,
        target_path: impl Into<String>,
        db: &impl ConnectionTrait,
    ) -> Result<Model, DbErr> {
        let target_model = Self::create_target(task_id, target_path);
        target_model.insert(db).await
    }
}

impl Entity {
    /// Find an existing target for the same task and path, or create a new one.
    pub async fn find_or_create(
        db: &impl ConnectionTrait,
        task_id: Uuid,
        target_path: impl Into<String> + Clone,
    ) -> Result<Model, DbErr> {
        let target_path_owned: String = target_path.clone().into();
        if let Some(target) = Entity::find()
            .filter(Column::TaskId.eq(task_id))
            .filter(Column::TargetPath.eq(target_path_owned.clone()))
            .one(db)
            .await?
        {
            return Ok(target);
        }

        // Handle potential race: unique(task_id, target_path) enforced in migration
        match Model::insert_target(task_id, target_path_owned.clone(), db).await {
            Ok(model) => Ok(model),
            Err(DbErr::Exec(RuntimeErr::SqlxError(sqlx::Error::Database(db_err)))) => {
                if let Some(code) = db_err.code() {
                    if code == Cow::from("23505") {
                        return Entity::find()
                            .filter(Column::TaskId.eq(task_id))
                            .filter(Column::TargetPath.eq(target_path_owned))
                            .one(db)
                            .await?
                            .ok_or_else(|| DbErr::RecordNotFound("target".into()));
                    }
                }
                Err(DbErr::Exec(RuntimeErr::SqlxError(sqlx::Error::Database(
                    db_err,
                ))))
            }
            Err(e) => Err(e),
        }
    }
}

/// Update target state and optional timing/error metadata.
pub async fn update_state(
    db: &impl ConnectionTrait,
    target_id: Uuid,
    state: TargetState,
    start_at: Option<DateTimeWithTimeZone>,
    end_at: Option<DateTimeWithTimeZone>,
    error_summary: Option<String>,
) -> Result<(), DbErr> {
    let mut active = ActiveModel {
        id: Set(target_id),
        state: Set(state),
        ..Default::default()
    };

    // When transitioning back to Building/Pending, clear stale end_at/error_summary
    match state {
        TargetState::Building | TargetState::Pending => {
            active.end_at = Set(None);
            active.error_summary = Set(None);
        }
        _ => {}
    }

    if let Some(start_at) = start_at {
        active.start_at = Set(Some(start_at));
    }

    if let Some(end_at) = end_at {
        active.end_at = Set(Some(end_at));
    }

    if let Some(summary) = error_summary {
        active.error_summary = Set(Some(summary));
    }

    active.update(db).await.map(|_| ())
}
