use crate::user;
use anyhow::Result;

mod db_handle;
pub use db_handle::DbHandle;

/// An application context caries state about the invoking user, references to resources like a
/// database connection pool, and other information used to process a request.
#[derive(Debug, Clone)]
pub struct Context {
    pub invoker: Invoker,

    /// Database executor - either a connection pool or a shared transaction.
    db: DbHandle,
}

impl Context {
    pub fn db(&self) -> &DbHandle {
        &self.db
    }
}

#[derive(Debug, Clone)]
pub enum Invoker {
    /// Represents an invocation by request of a user.
    User(user::User),

    /// Represents a system-level invocation, such as a background job or an internal service call.
    System(String),
}
