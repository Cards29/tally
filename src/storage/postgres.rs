use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Inserts an entry for the user and returns its time, set by the database.
///
/// # Errors
/// Returns an error if the insert fails.
pub async fn add_entry(pool: &PgPool, user_id: Uuid) -> Result<DateTime<Utc>> {
    sqlx::query_scalar!(
        "insert into entries (user_id) values ($1) returning created_at",
        user_id
    )
    .fetch_one(pool)
    .await
    .with_context(|| "failed to add entry")
}

/// Returns the user's entries, oldest first.
///
/// # Errors
/// Returns an error if the query fails.
pub async fn show_log(pool: &PgPool, user_id: Uuid) -> Result<Vec<DateTime<Utc>>> {
    sqlx::query_scalar!(
        "select created_at from entries where user_id = $1 order by created_at, id",
        user_id
    )
    .fetch_all(pool)
    .await
    .with_context(|| "failed to read entries")
}

/// Returns the user's newest entry, or `None` if they have none.
///
/// # Errors
/// Returns an error if the query fails.
pub async fn show_last(pool: &PgPool, user_id: Uuid) -> Result<Option<DateTime<Utc>>> {
    sqlx::query_scalar!(
        "select created_at from entries where user_id = $1 \
         order by created_at desc, id desc limit 1",
        user_id
    )
    .fetch_optional(pool)
    .await
    .with_context(|| "failed to read last entry")
}

/// Removes the user's newest entry. Does nothing if they have none.
///
/// One statement, so an entry added at the same moment is never lost.
///
/// # Errors
/// Returns an error if the delete fails.
pub async fn clear_last(pool: &PgPool, user_id: Uuid) -> Result<()> {
    sqlx::query!(
        "delete from entries where id = ( \
             select id from entries where user_id = $1 \
             order by created_at desc, id desc limit 1 \
         )",
        user_id
    )
    .execute(pool)
    .await
    .with_context(|| "failed to clear last entry")?;
    Ok(())
}

/// Removes all of the user's entries.
///
/// # Errors
/// Returns an error if the delete fails.
pub async fn clear_all(pool: &PgPool, user_id: Uuid) -> Result<()> {
    sqlx::query!("delete from entries where user_id = $1", user_id)
        .execute(pool)
        .await
        .with_context(|| "failed to clear entries")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Inserts a user with this handle and returns their id.
    async fn user(pool: &PgPool, handle: &str) -> Uuid {
        sqlx::query_scalar!(
            "insert into users (handle, display_name) values ($1, 'Test') returning id",
            handle
        )
        .fetch_one(pool)
        .await
        .expect("user should be insertable")
    }

    #[sqlx::test]
    async fn add_entry_appends_in_order(pool: PgPool) {
        let alice = user(&pool, "alice").await;

        let first = add_entry(&pool, alice)
            .await
            .expect("first entry should be added");
        let second = add_entry(&pool, alice)
            .await
            .expect("second entry should be added");

        let entries = show_log(&pool, alice)
            .await
            .expect("log should be readable");
        assert_eq!(entries, vec![first, second]);
    }

    #[sqlx::test]
    async fn show_last_returns_newest_entry(pool: PgPool) {
        let alice = user(&pool, "alice").await;
        add_entry(&pool, alice)
            .await
            .expect("first entry should be added");
        let second = add_entry(&pool, alice)
            .await
            .expect("second entry should be added");

        let last = show_last(&pool, alice)
            .await
            .expect("last entry should be readable");

        assert_eq!(last, Some(second));
    }

    #[sqlx::test]
    async fn show_last_is_none_when_log_empty(pool: PgPool) {
        let alice = user(&pool, "alice").await;

        let last = show_last(&pool, alice)
            .await
            .expect("last entry should be readable");

        assert_eq!(last, None);
    }

    #[sqlx::test]
    async fn clear_last_removes_only_newest_entry(pool: PgPool) {
        let alice = user(&pool, "alice").await;
        let first = add_entry(&pool, alice)
            .await
            .expect("first entry should be added");
        let second = add_entry(&pool, alice)
            .await
            .expect("second entry should be added");
        add_entry(&pool, alice)
            .await
            .expect("third entry should be added");

        clear_last(&pool, alice)
            .await
            .expect("last entry should be clearable");

        let entries = show_log(&pool, alice)
            .await
            .expect("log should be readable");
        assert_eq!(entries, vec![first, second]);
    }

    #[sqlx::test]
    async fn clear_all_empties_log(pool: PgPool) {
        let alice = user(&pool, "alice").await;
        add_entry(&pool, alice)
            .await
            .expect("first entry should be added");
        add_entry(&pool, alice)
            .await
            .expect("second entry should be added");

        clear_all(&pool, alice)
            .await
            .expect("log should be clearable");

        let entries = show_log(&pool, alice)
            .await
            .expect("log should be readable");
        assert!(entries.is_empty(), "expected no entries, got {entries:?}");
    }

    #[sqlx::test]
    async fn entries_are_isolated_per_user(pool: PgPool) {
        let alice = user(&pool, "alice").await;
        let bob = user(&pool, "bob").await;

        // Each user sees only their own entries.
        let a1 = add_entry(&pool, alice)
            .await
            .expect("alice's entry should be added");
        let b1 = add_entry(&pool, bob)
            .await
            .expect("bob's entry should be added");
        let entries = show_log(&pool, alice)
            .await
            .expect("alice's log should be readable");
        assert_eq!(entries, vec![a1]);
        let last = show_last(&pool, alice)
            .await
            .expect("alice's last should be readable");
        assert_eq!(last, Some(a1));

        // Alice clearing her last entry leaves Bob's newer entry alone.
        clear_last(&pool, alice)
            .await
            .expect("alice's last should be clearable");
        let entries = show_log(&pool, bob)
            .await
            .expect("bob's log should be readable");
        assert_eq!(entries, vec![b1]);

        // Bob clearing his log leaves Alice's entries alone.
        let a2 = add_entry(&pool, alice)
            .await
            .expect("alice's entry should be added");
        clear_all(&pool, bob)
            .await
            .expect("bob's log should be clearable");
        let entries = show_log(&pool, alice)
            .await
            .expect("alice's log should be readable");
        assert_eq!(entries, vec![a2]);
    }
}
