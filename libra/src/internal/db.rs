use crate::internal::model::*;
use crate::utils::path;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm::{
    ConnectionTrait, DbConn, DbErr, Schema, Statement, TransactionError, TransactionTrait,
};
use std::io;
use std::io::Error as IOError;
use std::io::ErrorKind;
use std::path::Path;

// #[cfg(not(test))]
// use tokio::sync::OnceCell;

/// Establish a connection to the database.
///  - `db_path` is the path to the SQLite database file.
/// - Returns a `DatabaseConnection` if successful, or an `IOError` if the database file does not exist.
#[allow(dead_code)]
pub async fn establish_connection(db_path: &str) -> Result<DatabaseConnection, IOError> {
    if !Path::new(db_path).exists() {
        return Err(IOError::new(
            ErrorKind::NotFound,
            "Database file does not exist.",
        ));
    }

    let mut option = ConnectOptions::new(format!("sqlite://{}", db_path));
    option.sqlx_logging(false); // TODO use better option
    Database::connect(option).await.map_err(|err| {
        IOError::new(
            ErrorKind::Other,
            format!("Database connection error: {:?}", err),
        )
    })
}
// #[cfg(not(test))]
// static DB_CONN: OnceCell<DbConn> = OnceCell::const_new();

// /// Get global database connection instance (singleton)
// #[cfg(not(test))]
// pub async fn get_db_conn_instance() -> &'static DbConn {
//     DB_CONN
//         .get_or_init(|| async { get_db_conn().await.unwrap() })
//         .await
// }

// #[cfg(test)]
use once_cell::sync::Lazy;
// #[cfg(test)]
use std::collections::HashMap;
//#[cfg(test)]
//use std::ops::Deref;
// #[cfg(test)]
use std::path::PathBuf;
// #[cfg(test)]
use tokio::sync::Mutex;

// In the test environment, use a `HashMap` to store database connections
// mapped by their working directories.
// change the value type from Box<DbConn> to &'static DbConn
// #[cfg(test)]
static TEST_DB_CONNECTIONS: Lazy<Mutex<HashMap<PathBuf, &'static DbConn>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

// #[cfg(test)]
#[allow(dead_code)]
fn leak_conn(conn: DbConn) -> &'static DbConn {
    let boxed = Box::new(conn);
    let static_ref = Box::leak(boxed);
    static_ref
}

/// In the test environment, each working directory should have its own database connection.
/// A global `HashMap` is used to store and manage these connections separately.
// #[cfg(test)]
pub async fn get_db_conn_instance() -> &'static DbConn {
    let current_dir = std::env::current_dir().unwrap();

    let mut connections = TEST_DB_CONNECTIONS.lock().await;

    if !connections.contains_key(&current_dir) {
        let conn = get_db_conn().await.unwrap();
        let boxed_conn = Box::new(conn);
        //connections.insert(current_dir.clone(), boxed_conn);
        connections.insert(current_dir.clone(), Box::leak(boxed_conn));
    }

    let boxed_conn = connections.get(&current_dir).unwrap();
    boxed_conn
    // leak_conn(boxed_conn.deref().clone())
}

/// Create a connection to the database of current repo: `.libra/libra.db`
async fn get_db_conn() -> io::Result<DatabaseConnection> {
    let db_path = path::database(); // for longer lifetime
    let db_path = db_path.to_str().unwrap();
    establish_connection(db_path).await
}

/// create table according to the Model
#[deprecated]
#[allow(dead_code)]
async fn setup_database_model(conn: &DatabaseConnection) -> Result<(), TransactionError<DbErr>> {
    // start a transaction
    conn.transaction::<_, _, DbErr>(|txn| {
        Box::pin(async move {
            let backend = txn.get_database_backend();
            let schema = Schema::new(backend);

            // reference table
            let table_create_statement = schema.create_table_from_entity(reference::Entity);
            txn.execute(backend.build(&table_create_statement)).await?;

            // config_section table
            let table_create_statement = schema.create_table_from_entity(config::Entity);
            txn.execute(backend.build(&table_create_statement)).await?;

            Ok(())
        })
    })
    .await
}

/// create table using sql in `src/sql/sqlite_20240331_init.sql`
async fn setup_database_sql(conn: &DatabaseConnection) -> Result<(), TransactionError<DbErr>> {
    conn.transaction::<_, _, DbErr>(|txn| {
        Box::pin(async move {
            let backend = txn.get_database_backend();

            // `include_str!` will expand the file while compiling, so `.sql` is not needed after that
            const SETUP_SQL: &str = include_str!("../../sql/sqlite_20240331_init.sql");
            txn.execute(Statement::from_string(backend, SETUP_SQL))
                .await?;
            Ok(())
        })
    })
    .await
}

/// Create a new SQLite database file at the specified path.
/// **should only be called in init or test**
/// - `db_path` is the path to the SQLite database file.
/// - Returns `Ok(())` if the database file was created and the schema was set up successfully.
/// - Returns an `IOError` if the database file already exists, or if there was an error creating the file or setting up the schema.
#[allow(dead_code)]
pub async fn create_database(db_path: &str) -> io::Result<DatabaseConnection> {
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

    // Connect to the new database and set up the schema.
    if let Ok(conn) = establish_connection(db_path).await {
        setup_database_sql(&conn).await.map_err(|err| {
            IOError::new(
                ErrorKind::Other,
                format!("Failed to setup database: {:?}", err),
            )
        })?;
        Ok(conn)
    } else {
        Err(IOError::new(
            ErrorKind::Other,
            "Failed to connect to new database.",
        ))
    }
}

#[cfg(test)]
mod tests {
    use sea_orm::{
        ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, EntityTrait, QueryFilter, Set,
    };
    use tests::reference::ConfigKind;

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
            let mut db_path = PathBuf::from("/tmp/test_db");
            if !db_path.exists() {
                let _ = fs::create_dir(&db_path);
            }
            db_path.push(name);
            db_path.to_str().unwrap().to_string();
            if db_path.exists() {
                let _ = fs::remove_file(&db_path);
            }
            let rt = TestDbPath(db_path.to_str().unwrap().to_string());
            create_database(rt.0.as_str()).await.unwrap();
            rt
        }
    }

    #[tokio::test]
    async fn test_create_database() {
        // didn't use TestDbPath, because TestDbPath use create_database to work.
        let db_path = "/tmp/test_create_database.db";
        if Path::new(db_path).exists() {
            fs::remove_file(db_path).unwrap();
        }
        let result = create_database(db_path).await;
        assert!(result.is_ok(), "create_database failed: {:?}", result);
        assert!(Path::new(db_path).exists());
        let result = create_database(db_path).await;
        assert!(result.is_err());
        // fs::remove_file(db_path).unwrap();
    }

    #[tokio::test]
    async fn test_insert_config() {
        // insert into config_entry & config_section, check foreign key constraint
        let test_db = TestDbPath::new("test_insert_config.db").await;
        let db_path = test_db.0.as_str();

        let conn = establish_connection(db_path).await.unwrap();
        // test insert config without name
        {
            let entries = [
                ("repositoryformatversion", "0"),
                ("filemode", "true"),
                ("bare", "false"),
                ("logallrefupdates", "true"),
            ];
            for (key, value) in entries.iter() {
                let entry = config::ActiveModel {
                    configuration: Set("core".to_string()),
                    name: Set(None),
                    key: Set(key.to_string()),
                    value: Set(value.to_string()),
                    ..Default::default()
                };
                let config = entry.save(&conn).await.unwrap();
                assert_eq!(config.key.unwrap(), key.to_string());
            }
            let result = config::Entity::find().all(&conn).await.unwrap();
            assert_eq!(result.len(), entries.len(), "config_section count is not 1");
        }
        // test insert config with name
        {
            let entry = config::ActiveModel {
                id: NotSet,
                configuration: Set("remote".to_string()),
                name: Set(Some("origin".to_string())),
                key: Set("url".to_string()),
                value: Set("https://localhost".to_string()),
            };
            let config = entry.save(&conn).await.unwrap();
            assert_ne!(config.id.unwrap(), 0);
        }

        // test search config
        {
            let result = config::Entity::find()
                .filter(config::Column::Configuration.eq("core"))
                .all(&conn)
                .await
                .unwrap();
            assert_eq!(result.len(), 4, "config_section count is not 5");
        }
    }

    #[tokio::test]
    async fn test_insert_reference() {
        // insert into reference, check foreign key constraint
        let test_db = TestDbPath::new("test_insert_reference.db").await;
        let db_path = test_db.0.as_str();

        let conn = establish_connection(db_path).await.unwrap();
        // test insert reference
        let entries = [
            (Some("master"), ConfigKind::Head, None, None), // attached head
            (None, ConfigKind::Head, Some("2019"), None),   // detached head
            (Some("master"), ConfigKind::Branch, Some("2019"), None), // local branch
            (Some("release1"), ConfigKind::Tag, Some("2019"), None), // tag (remote tag store same as local tag)
            (
                Some("main"),
                ConfigKind::Head,
                None,
                Some("origin".to_string()),
            ), // remote head
            (
                Some("main"),
                ConfigKind::Branch,
                Some("a"),
                Some("origin".to_string()),
            ),
        ];
        for (name, kind, commit, remote) in entries.iter() {
            let entry = reference::ActiveModel {
                name: Set(name.map(|s| s.to_string())),
                kind: Set(kind.clone()),
                commit: Set(commit.map(|s| s.to_string())),
                remote: Set(remote.clone()),
                ..Default::default()
            };
            let reference_entry = entry.save(&conn).await.unwrap();
            assert_eq!(reference_entry.name.unwrap(), name.map(|s| s.to_string()));
        }
    }

    #[tokio::test]
    async fn test_reference_check() {
        // test reference check
        let test_db = TestDbPath::new("test_reference_check.db").await;
        let db_path = test_db.0.as_str();

        let conn = establish_connection(db_path).await.unwrap();

        // test `remote`` can't be ''
        let entry = reference::ActiveModel {
            name: Set(Some("master".to_string())),
            kind: Set(ConfigKind::Head),
            commit: Set(Some("2019922235".to_string())),
            remote: Set(Some("".to_string())),
            ..Default::default()
        };
        let result = entry.save(&conn).await;
        assert!(
            result.is_err(),
            "reference check `remote` can't be '' failed"
        );

        // test `name`` can't be ''
        let entry = reference::ActiveModel {
            name: Set(Some("".to_string())),
            kind: Set(ConfigKind::Head),
            commit: Set(Some("2019922235".to_string())),
            remote: Set(Some("origin".to_string())),
            ..Default::default()
        };
        let result = entry.save(&conn).await;
        assert!(result.is_err(), "reference check `name` can't be '' failed");

        // test `remote` must be None for tag
        let entry = reference::ActiveModel {
            name: Set(Some("master".to_string())),
            kind: Set(ConfigKind::Tag),
            commit: Set(Some("2019922235".to_string())),
            remote: Set(Some("origin".to_string())),
            ..Default::default()
        };
        let result = entry.save(&conn).await;
        assert!(
            result.is_err(),
            "reference check `remote` must be None for tag failed"
        );

        // test (`name`, `type`) can't be duplicated when `remote` is None
        let entry = reference::ActiveModel {
            name: Set(Some("test_branch".to_string())),
            kind: Set(ConfigKind::Branch),
            ..Default::default()
        };
        let result = entry.clone().save(&conn).await;
        assert!(result.is_ok());
        let result = entry.save(&conn).await;
        assert!(result.is_err(), "reference check duplicated failed");

        // test (`name`, `type`) can't be duplicated when `remote` is not None
        let entry = reference::ActiveModel {
            name: Set(Some("test_branch".to_string())),
            kind: Set(ConfigKind::Branch),
            remote: Set(Some("origin".to_string())),
            ..Default::default()
        };
        let result = entry.clone().save(&conn).await;
        assert!(result.is_ok()); // not duplicated because remote is different
        let result = entry.save(&conn).await;
        assert!(result.is_err(), "reference check duplicated failed");
    }
}
