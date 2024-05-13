use crate::internal::db;
use crate::internal::model::{config, reference};
use crate::utils::util::{DATABASE, ROOT_DIR};
use sea_orm::{ActiveModelTrait, DbConn, DbErr, Set, TransactionTrait};
use std::{env, fs, io};

pub async fn execute() {
    init().await.unwrap();
}

/// Initialize a new Libra repository
#[allow(dead_code)]
pub async fn init() -> io::Result<()> {
    let cur_dir = env::current_dir()?;
    let root_dir = cur_dir.join(ROOT_DIR);
    if root_dir.exists() {
        println!("Already initialized - [{}]", root_dir.display());
        return Ok(());
    }

    // create .libra & sub-dirs
    let dirs = ["objects/pack", "objects/info", "info"];
    for dir in dirs {
        fs::create_dir_all(root_dir.join(dir))?;
    }
    // create info/exclude
    // `include_str!` includes the file content while compiling
    fs::write(
        root_dir.join("info/exclude"),
        include_str!("../../template/exclude"),
    )?;
    // create .libra/description
    fs::write(
        root_dir.join("description"),
        include_str!("../../template/description"),
    )?;

    // create database: .libra/libra.db
    let database = root_dir.join(DATABASE);
    let conn = db::create_database(database.to_str().unwrap()).await?;

    // create config table
    init_config(&conn).await.unwrap();

    // create HEAD
    reference::ActiveModel {
        name: Set(Some("master".to_owned())),
        kind: Set(reference::ConfigKind::Head),
        ..Default::default() // all others are `NotSet`
    }
    .insert(&conn)
    .await
    .unwrap();

    // set .libra as hidden
    set_dir_hidden(root_dir.to_str().unwrap())?;
    println!(
        "Initializing empty Libra repository in {}",
        root_dir.display()
    );
    Ok(())
}

async fn init_config(conn: &DbConn) -> Result<(), DbErr> {
    let txn = conn.begin().await?;

    #[cfg(not(target_os = "windows"))]
    let entries = [
        ("repositoryformatversion", "0"),
        ("filemode", "true"),
        ("bare", "false"),
        ("logallrefupdates", "true"),
    ];

    #[cfg(target_os = "windows")]
    let entries = [
        ("repositoryformatversion", "0"),
        ("filemode", "false"), // no filemode on windows
        ("bare", "false"),
        ("logallrefupdates", "true"),
        ("symlinks", "false"),  // no symlinks on windows
        ("ignorecase", "true"), // ignorecase on windows
    ];

    for (key, value) in entries {
        // tip: Set(None) == NotSet == default == NULL
        let entry = config::ActiveModel {
            configuration: Set("core".to_owned()),
            key: Set(key.to_owned()),
            value: Set(value.to_owned()),
            ..Default::default() // id & name NotSet
        };
        entry.insert(&txn).await?;
    }
    txn.commit().await?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn set_dir_hidden(dir: &str) -> io::Result<()> {
    use std::process::Command;
    Command::new("attrib").arg("+H").arg(dir).spawn()?.wait()?; // 等待命令执行完成
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn set_dir_hidden(_dir: &str) -> io::Result<()> {
    // on unix-like systems, dotfiles are hidden by default
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test;

    #[tokio::test]
    async fn test_init() {
        test::setup_without_libra();
        init().await.unwrap();
        // TODO check the result
    }
}
