mod custom_sql_types;
#[cfg(test)]
mod integration_tests;
mod load_swaps;
mod new_types;
mod save;
mod schema;
#[cfg(test)]
mod serialization_format_stability_tests;
mod swap;
mod swap_types;
#[macro_use]
pub mod with_swap_types;
embed_migrations!("./migrations");

pub use self::{
    load_swaps::{AcceptedSwap, LoadAcceptedSwap},
    save::*,
    swap::*,
    swap_types::*,
};

use crate::{
    db::custom_sql_types::Text,
    swap_protocols::{Role, SwapId},
};
use diesel::{self, prelude::*, sqlite::SqliteConnection};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::Arc,
};

/// This module provides persistent storage by way of Sqlite.

#[derive(Clone, derivative::Derivative)]
#[derivative(Debug)]
pub struct Sqlite {
    #[derivative(Debug = "ignore")]
    connection: Arc<async_std::sync::Mutex<SqliteConnection>>,
}

impl Sqlite {
    /// Return a handle that can be used to access the database.
    ///
    /// When this returns, an Sqlite database file 'cnd.sql' exists in 'dir', a
    /// successful connection to the database has been made, and the database
    /// migrations have been run.
    pub fn new_in_dir<D: AsRef<OsStr>>(dir: D) -> anyhow::Result<Self> {
        let dir = Path::new(&dir);
        let path = db_path_from_dir(dir);
        Sqlite::new(&path)
    }

    /// Return a handle that can be used to access the database.
    ///
    /// Reads or creates an SQLite database file at 'file'.  When this returns
    /// an Sqlite database exists, a successful connection to the database has
    /// been made, and the database migrations have been run.
    pub fn new(file: &Path) -> anyhow::Result<Self> {
        ensure_folder_tree_exists(file)?;

        let connection = SqliteConnection::establish(&format!("file:{}", file.display()))?;
        embedded_migrations::run(&connection)?;

        log::info!("SQLite database file: {}", file.display());

        Ok(Sqlite {
            connection: Arc::new(async_std::sync::Mutex::new(connection)),
        })
    }

    async fn do_in_transaction<F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: Fn(&SqliteConnection) -> Result<T, E>,
        E: From<diesel::result::Error>,
    {
        let guard = self.connection.lock().await;
        let connection = &*guard;

        let result = connection.transaction(|| f(&connection))?;

        Ok(result)
    }

    async fn role(&self, key: &SwapId) -> anyhow::Result<Role> {
        use self::schema::rfc003_swaps as swaps;

        let record: QueryableSwapRole = self
            .do_in_transaction(|connection| {
                let key = Text(key);

                swaps::table
                    .filter(swaps::swap_id.eq(key))
                    .select((swaps::swap_id, swaps::role))
                    .first(connection)
                    .optional()
            })
            .await?
            .ok_or(Error::SwapNotFound)?;

        Ok(*record.role)
    }
}

// Construct an absolute path to the database file using 'dir' as the base.
fn db_path_from_dir(dir: &Path) -> PathBuf {
    let path = dir.to_path_buf();
    path.join("cnd.sqlite")
}

fn ensure_folder_tree_exists(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    Ok(())
}

#[derive(Queryable, Debug, Clone, PartialEq)]
struct QueryableSwapRole {
    pub swap_id: Text<SwapId>,
    pub role: Text<Role>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("swap not found")]
    SwapNotFound,
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;
    use std::path::PathBuf;

    fn temp_db() -> PathBuf {
        let temp_file = tempfile::Builder::new()
            .suffix(".sqlite")
            .tempfile()
            .unwrap();

        temp_file.into_temp_path().to_path_buf()
    }

    #[test]
    fn can_create_a_new_temp_db() {
        let path = temp_db();

        let db = Sqlite::new(&path);

        assert_that(&db).is_ok();
    }

    #[test]
    fn given_no_database_exists_calling_new_creates_it() {
        let path = temp_db();
        // validate assumptions: the db does not exist yet
        assert_that(&path.as_path()).does_not_exist();

        let db = Sqlite::new(&path);

        assert_that(&db).is_ok();
        assert_that(&path.as_path()).exists();
    }

    #[test]
    fn given_db_in_non_existing_directory_tree_calling_new_creates_it() {
        let tempfile = tempfile::tempdir().unwrap();
        let mut path = PathBuf::new();

        path.push(tempfile);
        path.push("some_folder");
        path.push("i_dont_exist");
        path.push("database.sqlite");

        // validate assumptions:
        // 1. the db does not exist yet
        // 2. the parent folder does not exist yet
        assert_that(&path).does_not_exist();
        assert_that(&path.parent()).is_some().does_not_exist();

        let db = Sqlite::new(&path);

        assert_that(&db).is_ok();
        assert_that(&path).exists();
    }
}
