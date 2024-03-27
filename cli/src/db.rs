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
    let table_create_statement = schema.create_table_from_entity(super::model::reference::Entity);
    let table_create_result = conn.execute(backend.build(&table_create_statement)).await;
    match table_create_result {
        Ok(_) => (),
        Err(err) => return Err(err),
    }

    let table_create_statement =
        schema.create_table_from_entity(super::model::config_entry::Entity);
    let table_create_result = conn.execute(backend.build(&table_create_statement)).await;
    match table_create_result {
        Ok(_) => (),
        Err(err) => return Err(err),
    }

    let table_create_statement =
        schema.create_table_from_entity(super::model::config_section::Entity);
    let table_create_result = conn.execute(backend.build(&table_create_statement)).await;
    match table_create_result {
        Ok(_) => (),
        Err(err) => return Err(err),
    }
    Ok(())
}

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
    if let Ok(conn) = Database::connect(format!("sqlite://{}", db_path)).await {
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
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn test_create_database() {
        let db_path = "test_create_database.db";
        let _ = fs::remove_file(db_path);

        let result = create_database(db_path).await;
        assert!(result.is_ok());

        // let result = create_database(db_path).await;
        // assert!(result.is_err());

        // let _ = fs::remove_file(db_path);
    }
}
