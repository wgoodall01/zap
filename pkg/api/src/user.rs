//! This module defines how user logins work.
//!
//! - A User represents a single person.
//! - a FooLogin represents a login method for a user from the Foo service.
//!     - Each FooLogin is associated with a single User, but a User can have multiple FooLogins.

use crate::context::Context;
use crate::telegram_auth;
use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use rocket_okapi::okapi::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: Uuid,
    #[schemars(with = "String")]
    pub created_at: DateTime<Utc>,
    #[schemars(with = "String")]
    pub updated_at: DateTime<Utc>,

    pub name: String,
    pub photo_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginTg {
    pub id: Uuid,
    #[schemars(with = "String")]
    pub created_at: DateTime<Utc>,
    #[schemars(with = "String")]
    pub updated_at: DateTime<Utc>,

    pub user_id: Uuid, // references User
    pub tg_id: u64,

    pub username: String,
    pub first_name: String,
    pub last_name: Option<String>,
    pub photo_url: Option<String>,
}

impl User {
    /// Gets the user's UUID.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Gets the user's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets the user's photo URL, if any.
    pub fn photo_url(&self) -> Option<&str> {
        self.photo_url.as_deref()
    }

    /// Gets the appropriate User for this LoginTelegram, if one doesn't exist already.
    pub async fn from_telegram(tg_user: &telegram_auth::TgUser, ctx: &Context) -> Result<User> {
        // Fast path: get an existing user by finding related LoginTg record.
        let existing_user: Option<User> = sqlx::query_as!(
            User,
            r#"
            select u.* from "user" u
            where exists(
                select * from login_tg ltg
                where ltg.user_id = u.id and ltg.tg_id = $1
            )
            "#,
            tg_user.user_id as i64,
        )
        .fetch_optional(ctx)
        .await
        .context("Failed to check for existing user")?;
        if let Some(u) = existing_user {
            // The user already exists, log in as that user.
            return Ok(u);
        }

        // Start a transaction to create a new user and associated LoginTg record.
        let ctx = ctx.in_txn().await?;

        // Create a new user record
        let user_id = Uuid::now_v7();
        let user: User = sqlx::query_as!(
            User,
            r#"
            insert into "user" (id, name, photo_url)
            values ($1, $2, $3)
            returning *
            "#,
            user_id,
            tg_user.name.clone(),
            tg_user.photo_url.clone()
        )
        .fetch_one(&ctx)
        .await
        .context("Failed to create new user")?;

        // Create the associated LoginTg record
        let login_id = Uuid::now_v7();
        sqlx::query!(
            r#"
            insert into login_tg (id, user_id, tg_id, username, first_name, last_name, photo_url)
            values ($1, $2, $3, $4, $5, $6, $7)
            "#,
            login_id,
            user_id,
            tg_user.user_id as i64,
            tg_user.tg_username,
            tg_user.name,
            None::<String>,
            tg_user.photo_url
        )
        .execute(&ctx)
        .await
        .context("Failed to create LoginTg record")?;

        ctx.commit()
            .await
            .context("Failed to commit Telegram user registration transaction")?;

        Ok(user)
    }
}
