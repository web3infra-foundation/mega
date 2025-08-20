use crate::internal::config;
use crate::internal::db::get_db_conn_instance;
use crate::internal::head::Head;
use crate::internal::model::reflog;
use crate::internal::model::reflog::{ActiveModel, Model};
use sea_orm::{ActiveModelTrait, DatabaseTransaction, EntityTrait, QueryFilter, QueryOrder, Set, TransactionTrait};
use sea_orm::{ColumnTrait, ConnectionTrait, DbBackend, DbErr, Statement, TransactionError};
use std::fmt::{Debug, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::time::{SystemTime, UNIX_EPOCH};

const HEAD: &str = "HEAD";

#[derive(Debug)]
pub struct ReflogContext {
    pub old_oid: String,
    pub new_oid: String,
    pub action: ReflogAction,
}

pub enum ReflogError {
    DatabaseError(DbErr),
    TransactionError(TransactionError<DbErr>),
}

impl Display for ReflogError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:?}", self)
    }
}

impl Debug for ReflogError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError(e) => write!(f, "Database error: {}", e),
            Self::TransactionError(e) => write!(f, "Transaction error: {}", e),
        }
    }
}

impl From<DbErr> for ReflogError {
    fn from(err: DbErr) -> Self {
        ReflogError::DatabaseError(err)
    }
}

impl From<TransactionError<DbErr>> for ReflogError {
    fn from(err: TransactionError<DbErr>) -> Self {
        ReflogError::TransactionError(err)
    }
}
impl Display for ReflogContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.action {
            ReflogAction::Commit { message } => write!(f, "{}", message.lines().next().unwrap_or("")),
            ReflogAction::Switch { from, to } => write!(f, "moving from {from} to {to}"),
            ReflogAction::Reset { target } => write!(f, "moving to {target}"),
            ReflogAction::Merge { branch, policy } => write!(f, "merge {branch}:{policy}"),
            ReflogAction::Pull => todo!(),
            ReflogAction::Clone { from } => write!(f, "from {from}"),
        }
    }
}

#[derive(Debug)]
pub enum ReflogAction {
    Commit { message: String },
    Reset { target: String },
    Switch { from: String, to: String },
    Merge { branch: String, policy: String },
    Pull,
    Clone { from: String },
}

#[derive(Copy, Clone)]
pub enum ReflogActionKind {
    Commit,
    Reset,
    Switch,
    Merge,
    Pull,
    Clone,
}

impl Display for ReflogActionKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Commit => write!(f, "commit"),
            Self::Reset => write!(f, "reset"),
            Self::Switch => write!(f, "switch"),
            Self::Merge => write!(f, "merge"),
            Self::Pull => write!(f, "pull"),
            Self::Clone => write!(f, "clone"),
        }
    }
}

impl ReflogAction {
    fn kind(&self) -> ReflogActionKind {
        match self {
            Self::Commit { .. } => ReflogActionKind::Commit,
            Self::Reset { .. } => ReflogActionKind::Reset,
            Self::Switch { .. } => ReflogActionKind::Switch,
            Self::Merge { .. } => ReflogActionKind::Merge,
            Self::Pull => ReflogActionKind::Pull,
            Self::Clone { .. } => ReflogActionKind::Clone,
        }
    }
}

pub struct Reflog;

impl Reflog {
    /// insert a reflog record.
    /// see `ReflogContext`
    pub async fn insert(db: &DatabaseTransaction, context: ReflogContext) -> Result<(), ReflogError> {
        ensure_reflog_table_exists(db).await?;
        let kind = context.action.kind();
        let head = Head::current_with_conn(db).await;

        let ref_name = match head {
            Head::Branch(name) => Some(name),
            Head::Detached(_) => None,
        };

        // considering that there are many commands that have not yet used user configs,
        // we just set default user info.
        let name = config::Config::get_with_conn(db, "user", None, "name")
            .await
            .unwrap_or("mega".to_string());
        let email = config::Config::get_with_conn(db, "user", None, "email")
            .await
            .unwrap_or("admin@mega.org".to_string());
        let message = context.to_string();

        let mut model = ActiveModel {
            ref_name: Set(HEAD.to_string()),
            old_oid: Set(context.old_oid),
            new_oid: Set(context.new_oid),
            action: Set(kind.to_string()),
            committer_name: Set(name),
            committer_email: Set(email),
            timestamp: Set(timestamp_seconds()),
            message: Set(message),
            ..Default::default()
        };

        match ref_name {
            Some(ref_name) => {
                model.clone().save(db).await?;

                if !matches!(kind, ReflogActionKind::Switch) {
                    model.ref_name = Set(ref_name.to_string());
                    model.save(db).await?;
                }
            }
            None => {
                model.save(db).await?;
            }
        }
        Ok(())
    }

    pub async fn find_all<C: ConnectionTrait>(db: &C, ref_name: &str) -> Vec<Model> {
        reflog::Entity::find()
            .filter(reflog::Column::RefName.eq(ref_name))
            .order_by_desc(reflog::Column::Timestamp)
            .all(db)
            .await
            .unwrap()
    }

    pub async fn find_one<C: ConnectionTrait>(db: &C, ref_name: &str) -> Result<Option<Model>, ReflogError> {
        Ok(reflog::Entity::find()
            .filter(reflog::Column::RefName.eq(ref_name))
            .order_by_desc(reflog::Column::Timestamp)
            .one(db)
            .await?)
    }
}

fn timestamp_seconds() -> i64 {
    let now = SystemTime::now();
    let since_the_epoch = now.duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    since_the_epoch.as_secs() as i64
}

/// Executes a database operation within a transaction and records a reflog entry upon success.
///
/// This function acts as a safe, atomic wrapper for any operation that needs to be
/// recorded in the reflog. It ensures that the core operation and the creation of its
/// corresponding reflog entry either both succeed and are committed, or both fail and
/// are rolled back. This prevents inconsistent states where an action is performed
/// but not logged.
///
/// # Example
///
/// Here is how you would use `with_reflog` to wrap a `commit` operation.
///
/// ```rust,ignore
/// // 1. First, prepare the context for the reflog entry.
/// let reflog_context = ReflogContext {
///     old_oid: "previous_commit_hash".to_string(),
///     new_oid: "new_commit_hash".to_string(),
///     action: ReflogAction::Commit {
///         message: message.to_string(),
///     }
/// };
///
/// // 2. Define the core database operation as an async closure.
/// //    Note that all DB calls inside MUST use the provided `txn` handle.
/// let core_operation = |txn: &DatabaseTransaction| Box::pin(async move {
///     // This is where you move the branch pointer, update HEAD, etc.
///     // IMPORTANT: Use `_with_conn` variants of your helper functions.
///     Branch::update_branch_with_conn(txn, "main", "new_commit_hash", None).await;
///     Head::update_with_conn(txn, Head::Branch("main".to_string()), None).await;
///
///     // The closure must return a Result compatible with DbErr.
///     Ok(())
/// });
///
/// // 3. Execute the wrapper.
/// match with_reflog(reflog_context, core_operation).await {
///     Ok(_) => println!("Commit and reflog recorded successfully."),
///     Err(e) => eprintln!("Operation failed: {:?}", e),
/// }
/// ```
pub async fn with_reflog<F>(
    context: ReflogContext,
    operation: F,
) -> Result<(), ReflogError>
where
        for<'b> F: FnOnce(&'b DatabaseTransaction) -> Pin<Box<dyn Future<Output = Result<(), DbErr>> + Send + 'b>>,
        F: Send + 'static,
{
    let db = get_db_conn_instance().await;
    db.transaction(|txn| {
        Box::pin(async move {
            operation(txn).await
                .map_err(ReflogError::from)?;
            Reflog::insert(txn, context).await?;
            Ok::<_, ReflogError>(())
        })
    })
        .await
        .map_err(|err| match err {
            TransactionError::Connection(err) => ReflogError::from(err),
            TransactionError::Transaction(err) => err,
        })
}

/// Check whether the current libra repo have a `reflog` table
async fn reflog_table_exists<C: ConnectionTrait>(db_conn: &C) -> Result<bool, ReflogError> {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Sqlite,
        r#"
            SELECT COUNT(*)
            FROM sqlite_master
            WHERE type='table' AND name=?;
        "#,
        ["reflog".into()]
    );

    if let Some(result) = db_conn.query_one(stmt).await? {
        let count = result.try_get_by_index(0).unwrap_or(0);
        if count == 0 {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Ensures that the 'reflog' table and its associated indexes exist in the database.
/// If they do not exist, they will be created.
async fn ensure_reflog_table_exists<C: ConnectionTrait>(db: &C) -> Result<(), ReflogError> {
    if reflog_table_exists(db).await? {
        return Ok(());
    }

    println!("Warning: The current libra repo does not have a `reflog` table, creating one...");
    let create_table_stmt = Statement::from_string(
        DbBackend::Sqlite,
        r#"
            CREATE TABLE IF NOT EXISTS `reflog` (
                `id`              INTEGER PRIMARY KEY AUTOINCREMENT,
                `ref_name`        TEXT NOT NULL,
                `old_oid`         TEXT NOT NULL,
                `new_oid`         TEXT NOT NULL,
                `committer_name`  TEXT NOT NULL,
                `committer_email` TEXT NOT NULL,
                `timestamp`       INTEGER NOT NULL,
                `action`          TEXT NOT NULL,
                `message`         TEXT NOT NULL
            );
        "#.to_string(),
    );

    db.execute(create_table_stmt).await?;

    let create_index_stmt = Statement::from_string(
        DbBackend::Sqlite,
        r#"
            CREATE INDEX IF NOT EXISTS idx_ref_name_timestamp ON `reflog`(`ref_name`, `timestamp`);
        "#.to_string(),
    );

    db.execute(create_index_stmt).await?;
    Ok(())
}