use common::errors::MegaError;

use super::service::MonoApiService;
use crate::model::note::NoteShowResponse;

impl MonoApiService {
    pub async fn get_note_sync_state(&self, id: i32) -> Result<NoteShowResponse, MegaError> {
        let note = self
            .storage
            .note_storage()
            .get_note_by_id(id.into())
            .await?
            .ok_or_else(|| MegaError::NotFound(format!("Note {id} not found")))?;

        Ok(NoteShowResponse {
            public_id: note.public_id,
            description_schema_version: note.description_schema_version,
            description_state: match &note.description_state {
                Some(state) if !state.is_empty() => Some(state.clone()),
                _ => None,
            },
            description_html: match &note.description_html {
                Some(html) if !html.is_empty() => html.clone(),
                _ => String::new(),
            },
        })
    }

    pub async fn update_note_sync_state(
        &self,
        id: i32,
        description_html: &str,
        description_state: &str,
        description_schema_version: i32,
    ) -> Result<(), MegaError> {
        let note = self
            .storage
            .note_storage()
            .get_note_by_id(id.into())
            .await?
            .ok_or_else(|| MegaError::NotFound(format!("Note {id} not found")))?;

        if description_schema_version < note.description_schema_version {
            return Err(MegaError::Other(format!(
                "Invalid schema version: provided ({description_schema_version}) is older than current ({})",
                note.description_schema_version
            )));
        }

        self.storage
            .note_storage()
            .update_note(
                id,
                description_html,
                description_state,
                description_schema_version,
            )
            .await?;
        Ok(())
    }
}
