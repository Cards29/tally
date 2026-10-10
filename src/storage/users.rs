use anyhow::{Context, Result, bail};
use sqlx::PgPool;

use crate::storage::tokens;

/// Inserts a user with the `admin` role.
///
/// # Errors
/// Returns an error if the handle is taken or invalid, or the insert fails.
pub async fn create_admin(pool: &PgPool, handle: &str, display_name: &str) -> Result<()> {
    sqlx::query!(
        "insert into users (handle, display_name, role) values ($1, $2, 'admin')",
        handle,
        display_name
    )
    .execute(pool)
    .await
    .with_context(|| format!("failed to create admin {handle}"))?;
    Ok(())
}

/// Adds a device session for the user with this handle and returns its token.
///
/// Only the token's hash is stored, so the caller sees the token this one time.
///
/// # Errors
/// Returns an error if no user has this handle, or the token can't be generated or stored.
pub async fn create_device_token(pool: &PgPool, handle: &str, name: &str) -> Result<String> {
    let token = tokens::generate()?;

    let result = sqlx::query!(
        "insert into sessions (user_id, token_hash, kind, name) \
         select id, $2, 'device', $3 from users where handle = $1",
        handle,
        tokens::hash(&token),
        name
    )
    .execute(pool)
    .await
    .with_context(|| format!("failed to create device token for {handle}"))?;
    if result.rows_affected() == 0 {
        bail!("no user with handle {handle}");
    }

    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn create_admin_inserts_admin_user(pool: PgPool) {
        create_admin(&pool, "alice", "Alice")
            .await
            .expect("admin should be created");

        let user = sqlx::query!("select display_name, role from users where handle = 'alice'")
            .fetch_one(&pool)
            .await
            .expect("alice should exist");
        assert_eq!(user.display_name, "Alice");
        assert_eq!(user.role, "admin");
    }

    #[sqlx::test]
    async fn create_device_token_stores_hash_of_returned_token(pool: PgPool) {
        create_admin(&pool, "alice", "Alice")
            .await
            .expect("admin should be created");

        let token = create_device_token(&pool, "alice", "Pixel shortcut")
            .await
            .expect("token should be created");

        let session = sqlx::query!(
            "select kind, name from sessions \
             where token_hash = $1 and user_id = (select id from users where handle = 'alice')",
            tokens::hash(&token)
        )
        .fetch_one(&pool)
        .await
        .expect("alice should have a session for the token");
        assert_eq!(session.kind, "device");
        assert_eq!(session.name.as_deref(), Some("Pixel shortcut"));
    }

    #[sqlx::test]
    async fn create_device_token_fails_for_unknown_handle(pool: PgPool) {
        let error = create_device_token(&pool, "nobody", "Pixel shortcut")
            .await
            .expect_err("unknown handle should fail");

        assert_eq!(error.to_string(), "no user with handle nobody");
    }
}
