use std::ops::Deref;

use callisto::notes;
use common::errors::MegaError;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, IntoActiveModel};

use crate::storage::base_storage::{BaseStorage, StorageConnector};

#[derive(Clone)]
pub struct NoteStorage {
    pub base: BaseStorage,
}

impl Deref for NoteStorage {
    type Target = BaseStorage;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl NoteStorage {
    pub async fn get_note_by_id(&self, id: i64) -> Result<Option<notes::Model>, MegaError> {
        let model = notes::Entity::find_by_id(id)
            .one(self.get_connection())
            .await?;
        Ok(model)
    }
    pub async fn save_note(&self, note: notes::Model) -> Result<(), MegaError> {
        let a_model = note.into_active_model();
        a_model.insert(self.get_connection()).await?;
        Ok(())
    }
    pub async fn save_note_context(
        &self,
        payload: CreateNotePayload,
    ) -> Result<Option<notes::Model>, MegaError> {
        let note_active_model = notes::ActiveModel {
            public_id: Set(payload.public_id),
            organization_membership_id: Set(payload.organization_membership_id),
            title: Set(payload.title),
            description_html: Set(payload.description_html),
            description_state: Set(payload.description_state),
            project_id: Set(payload.project_id),
            visibility: Set(payload.visibility.unwrap_or(0)),
            ..Default::default()
        };

        let save_note = note_active_model.insert(self.get_connection()).await;
        match save_note {
            Ok(model) => Ok(Some(model)),
            Err(e) => Err(MegaError::with_message(format!(
                "Failed to save note: {}",
                e
            ))),
        }
    }

    pub async fn update_note(
        &self,
        id: i32,
        description_html: &str,
        description_state: &str,
        description_schema_version: i32,
    ) -> Result<i32, MegaError> {
        let model = notes::Entity::find_by_id(id)
            .one(self.get_connection())
            .await?
            .ok_or_else(|| MegaError::with_message(format!("Note with ID {} not found", id)))?;        let mut active_model: notes::ActiveModel = model.into();
        active_model.description_html = Set(Some(description_html.to_string()));
        active_model.description_state = Set(Some(description_state.to_string()));
        active_model.description_schema_version = Set(description_schema_version);
        let updated_model = active_model.update(self.get_connection()).await?;
        Ok(updated_model.id as i32)
    }
}

#[derive(Clone, Debug)]
pub struct CreateNotePayload {
    pub public_id: String,
    pub organization_membership_id: i64,

    pub title: Option<String>,
    pub description_html: Option<String>,
    pub description_state: Option<String>,

    pub project_id: Option<i64>,
    pub visibility: Option<i32>,
}
