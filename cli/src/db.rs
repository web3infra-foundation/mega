use crate::model::*;
use sea_orm::{ConnectionTrait, Schema};
use sea_orm::{Database, DatabaseConnection};
use std::io::Error as IOError;
use std::io::ErrorKind;
use std::path::Path;
/// Establish a connection to the database.
///  - `db_path` is the path to the SQLite database file.
/// - Returns a `DatabaseConnection` if successful, or an `IOError` if the database file does not exist.
pub async fn establish_connection(db_path: &str) -> Result<DatabaseConnection, IOError> {
    if !Path::new(db_path).exists() {
        return Err(IOError::new(
            ErrorKind::NotFound,
            "Database file does not exist.",
        ));
    }

    Database::connect(format!("sqlite://{}", db_path))
        .await
        .map_err(|err| {
            IOError::new(
                ErrorKind::Other,
                format!("Database connection error: {:?}", err),
            )
        })
}

/// create table according to the Model
async fn setup_database(conn: &DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
    let backend = conn.get_database_backend();
    let schema = Schema::new(backend);

    // reference table
    let table_create_statement = schema.create_table_from_entity(reference::Entity);
    let table_create_result = conn.execute(backend.build(&table_create_statement)).await;
    match table_create_result {
        Ok(_) => (),
        Err(err) => return Err(err),
    }

    // config_section table
    let table_create_statement = schema.create_table_from_entity(config_section::Entity);
    let table_create_result = conn.execute(backend.build(&table_create_statement)).await;
    match table_create_result {
        Ok(_) => (),
        Err(err) => return Err(err),
    }

    // config_entry table
    let table_create_statement = schema.create_table_from_entity(config_entry::Entity);
    let table_create_result = conn.execute(backend.build(&table_create_statement)).await;
    match table_create_result {
        Ok(_) => (),
        Err(err) => return Err(err),
    }
    Ok(())
}

/// Create a new SQLite database file at the specified path.
/// **should only be called in init or test**
/// - `db_path` is the path to the SQLite database file.
/// - Returns `Ok(())` if the database file was created and the schema was setup successfully.
/// - Returns an `IOError` if the database file already exists, or if there was an error creating the file or setting up the schema.
pub async fn create_database(db_path: &str) -> Result<(), IOError> {
    if Path::new(db_path).exists() {
        return Err(IOError::new(
            ErrorKind::AlreadyExists,
            "Database file already exists.",
        ));
    }

    std::fs::File::create(db_path).map_err(|err| {
        IOError::new(
            ErrorKind::Other,
            format!("Failed to create database file: {:?}", err),
        )
    })?;

    // Connect to the new database and setup the schema.
    if let Ok(conn) = establish_connection(&db_path).await {
        setup_database(&conn).await.map_err(|err| {
            IOError::new(
                ErrorKind::Other,
                format!("Failed to setup database: {:?}", err),
            )
        })
    } else {
        Err(IOError::new(
            ErrorKind::Other,
            "Failed to connect to new database.",
        ))
    }
}

#[cfg(test)]
mod tests {
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    use super::*;
    use std::{fs, path::PathBuf};

    /// TestDbPath is a helper struct create and delete test database file
    struct TestDbPath(String);
    impl Drop for TestDbPath {
        fn drop(&mut self) {
            if Path::new(&self.0).exists() {
                let _ = fs::remove_file(&self.0);
            }
        }
    }
    impl TestDbPath {
        async fn new(name: &str) -> Self {
            let mut db_path = PathBuf::from("/tmp/testdb");
            if !db_path.exists() {
                let _ = fs::create_dir(&db_path);
            }
            db_path.push(name);
            db_path.to_str().unwrap().to_string();
            if db_path.exists() {
                let _ = fs::remove_file(&db_path);
            }
            let rt = TestDbPath(db_path.to_str().unwrap().to_string());
            let _ = create_database(rt.0.as_str()).await;
            rt
        }
    }

    #[tokio::test]
    async fn test_create_database() {
        // didn't use TestDbPath, because TestDbPath use create_database to work.
        let db_path = "/tmp/test_create_database.db";
        let result = create_database(db_path).await;
        assert!(result.is_ok(), "create_database failed: {:?}", result);
        assert!(Path::new(db_path).exists());
        let result = create_database(db_path).await;
        assert!(result.is_err());
        fs::remove_file(db_path).unwrap();
    }

    #[tokio::test]
    async fn test_insert_config_entry() {
        // insert into config_entry & config_section, check foreign key constraint
        let test_db = TestDbPath::new("test_insert_config_entry.db").await;
        let db_path = test_db.0.as_str();

        let conn = establish_connection(db_path).await.unwrap();
        // （section_name, unique_name) is unique
        let config_section = config_section::ActiveModel {
            section_name: Set("core".to_string()),
            ..Default::default()
        };
        let config_section = config_section.save(&conn).await.unwrap();
        let entries = [
            ("repositoryformatversion".to_string(), "0".to_string()),
            ("filemode".to_string(), "true".to_string()),
            ("bare".to_string(), "false".to_string()),
            ("logallrefupdates".to_string(), "true".to_string()),
        ];
        for (key, value) in entries.iter() {
            let config_entry = config_entry::ActiveModel {
                section_id: Set(*config_section.section_id.as_ref()),
                key: Set(key.to_string()),
                value: Set(value.to_string()),
                ..Default::default()
            };
            let config_entry = config_entry.save(&conn).await.unwrap();
            assert_eq!(config_entry.section_id, config_section.section_id);
        }
        let result = config_section::Entity::find().all(&conn).await.unwrap();
        assert_eq!(result.len(), 1, "config_section count is not 1");
        let result = config_entry::Entity::find().all(&conn).await.unwrap();
        assert_eq!(result.len(), entries.len(), "config_entry count error");
    }
}
