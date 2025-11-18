use model::tugraph_model::{Program, UProgram};

use crate::db::DBHandler;

pub struct DataPacker {
    db: DBHandler,
}

impl DataPacker {
    pub async fn new() -> Self {
        let db = DBHandler::connect().await.unwrap();

        db.create_tables().await.unwrap();
        Self { db }
    }

    pub async fn pack_into_db(
        &self,
        program: Program,
        uprogram: UProgram,
        //versions: Vec<crate::VersionInfo>,
    ) {
        self.db
            .insert_program_data(program, uprogram)
            .await
            .unwrap();
    }
}
