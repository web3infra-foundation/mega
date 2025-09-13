use crate::{
    data_packer::DataPacker,
    data_reader::{DataReader, DataReaderTrait},
};

pub struct Transporter {
    pub reader: DataReader,
    pub packer: DataPacker,
}

impl Transporter {
    pub async fn new(uri: &str, user: &str, password: &str, db: &str) -> Self {
        Self {
            reader: DataReader::new(uri, user, password, db).await.unwrap(),
            packer: DataPacker::new().await,
        }
    }

    pub async fn transport_data(&mut self) -> Result<(), ()> {
        tracing::info!("Start to pack the data");
        let ids = self.reader.get_all_programs_id().await;
        for id in ids {
            tracing::info!("id:{}", id);
            let program: model::tugraph_model::Program =
                self.reader.get_program(&id).await.unwrap();
            let (uprogram, islib): (model::tugraph_model::UProgram, bool) =
                self.reader.get_type(&id).await.unwrap();
            let versions: Vec<crate::VersionInfo> =
                self.reader.get_versions(&id, islib).await.unwrap();

            self.packer.pack_into_db(program, uprogram, versions).await;
        }
        tracing::info!("finish to pack the data");
        Ok(())
    }
}
