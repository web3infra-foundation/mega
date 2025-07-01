use std::{collections::HashMap, path::PathBuf};

use qdrant_client::{qdrant::PointStruct, Payload};

#[derive(Debug, Clone)]
pub struct CodeItem {
    pub name: String,
    pub content: String,
    pub item_type: ItemType,
    pub file_path: PathBuf,
    pub vector: Vec<f64>,
}

#[derive(Debug, Clone)]
pub enum ItemType {
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Type,
    Const,
    Static,
    Module,
}

impl CodeItem {
    pub fn to_qdrant_point(&self, id: u64) -> PointStruct {
        // Convert f64 vector to f32 (Qdrant requirement)
        let vector: Vec<f32> = self.vector.iter().map(|&x| x as f32).collect();

        // Build payload metadata
        let payload: HashMap<String, qdrant_client::qdrant::Value> = [
            ("name".to_string(), self.name.clone().into()),
            ("content".to_string(), self.content.clone().into()),
            ("type".to_string(), format!("{:?}", self.item_type).into()),
            (
                "file_path".to_string(),
                self.file_path.to_string_lossy().into_owned().into(),
            ),
        ]
        .into_iter()
        .collect();

        let payload = Payload::from(payload);
        PointStruct::new(id, vector, payload)
    }
}
