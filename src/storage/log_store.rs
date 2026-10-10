use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::storage::{log, postgres};

/// Where log entries live. Each method picks the store and passes the call on.
#[derive(Clone)]
pub enum LogStore {
    /// Plain text file at this path. Ignores `user_id`: every user shares one log.
    File(String),
    /// The `entries` table, scoped by `user_id`.
    Postgres(PgPool),
}

impl LogStore {
    /// Adds an entry stamped with the current time and returns it.
    ///
    /// # Errors
    /// Returns an error if the store can't be written.
    pub async fn add_entry(&self, user_id: Uuid) -> Result<DateTime<Utc>> {
        match self {
            Self::File(file_name) => log::add_entry(file_name),
            Self::Postgres(pool) => postgres::add_entry(pool, user_id).await,
        }
    }

    /// Returns every entry, oldest first.
    ///
    /// # Errors
    /// Returns an error if the store can't be read.
    pub async fn show_log(&self, user_id: Uuid) -> Result<Vec<DateTime<Utc>>> {
        match self {
            Self::File(file_name) => log::show_log(file_name),
            Self::Postgres(pool) => postgres::show_log(pool, user_id).await,
        }
    }

    /// Returns the newest entry, or `None` if the log is empty.
    ///
    /// # Errors
    /// Returns an error if the store can't be read.
    pub async fn show_last(&self, user_id: Uuid) -> Result<Option<DateTime<Utc>>> {
        match self {
            Self::File(file_name) => log::show_last(file_name),
            Self::Postgres(pool) => postgres::show_last(pool, user_id).await,
        }
    }

    /// Removes the newest entry. Does nothing if the log is empty.
    ///
    /// # Errors
    /// Returns an error if the store can't be read or written.
    pub async fn clear_last(&self, user_id: Uuid) -> Result<()> {
        match self {
            Self::File(file_name) => log::clear_last(file_name),
            Self::Postgres(pool) => postgres::clear_last(pool, user_id).await,
        }
    }

    /// Removes every entry.
    ///
    /// # Errors
    /// Returns an error if the store can't be written.
    pub async fn clear_all(&self, user_id: Uuid) -> Result<()> {
        match self {
            Self::File(file_name) => log::clear_all(file_name),
            Self::Postgres(pool) => postgres::clear_all(pool, user_id).await,
        }
    }
}
