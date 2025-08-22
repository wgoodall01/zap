//! This module defines how user logins work.
//!
//! - A User represents a single person.
//! - a FooLogin represents a login method for a user from the Foo service.
//!     - Each FooLogin is associated with a single User, but a User can have multiple FooLogins.

use crate::context::Context;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// An application user.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    id: Uuid,
    name: String,
    photo_url: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
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
}

/// A telegram user login.
struct LoginTelegram {
    pub tg_id: u64,

    pub username: String,
    pub first_name: String,
    pub last_name: Option<String>,
    pub photo_url: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl LoginTelegram {
    /// Gets the appropriate User for this LoginTelegram, if one doesn't exist already.
    pub fn login_or_register(&self, ctx: &Context) -> Result<User> {
        todo!("implement login_or_register")
    }
}
