use crate::auth;
use crate::error::ApiError;
use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Transaction};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use utoipa::ToSchema;
use uuid::Uuid;

/// An application context caries state about the invoking user, references to resources like a
/// database connection pool, and other information used to process a request.
#[derive(Debug, Clone)]
pub struct Context {
    pub invoker: Invoker,

    /// Database pool.
    db_pool: sqlx::Pool<sqlx::Postgres>,

    /// Optional database transaction. If present, all database operations will use this transaction.
    /// If None, operations will use the connection pool directly.
    txn: Arc<Mutex<Option<Transaction<'static, Postgres>>>>,
}

impl Context {
    /// Create a new Context with a named System invoker.
    pub fn new_system(system_id: &'static str, db_pool: sqlx::Pool<sqlx::Postgres>) -> Self {
        Self {
            invoker: Invoker::from_system(system_id),
            db_pool,
            txn: Arc::new(Mutex::new(None)),
        }
    }

    /// Create a new Context with the given invoker and database pool.
    pub fn new_user(user: &auth::User, db_pool: sqlx::Pool<sqlx::Postgres>) -> Self {
        Self {
            invoker: Invoker::from_user(user),
            db_pool,
            txn: Arc::new(Mutex::new(None)),
        }
    }

    /// Commit the current transaction, if it exists.
    #[tracing::instrument(name = "Context::commit", skip(self))]
    pub async fn commit(&self) -> Result<(), sqlx::Error> {
        let mut guard = self.txn.lock().await;
        if let Some(txn) = guard.take() {
            txn.commit().await
        } else {
            Ok(())
        }
    }

    /// Create a child context which runs in a single database transaction.
    #[tracing::instrument(name = "Context::in_txn", skip(self))]
    pub async fn in_txn(&self) -> Result<Context, sqlx::Error> {
        let guard = self.txn.lock().await;
        if guard.is_some() {
            // If we already have a transaction, we can't create a nested one with sqlx
            // Return an error or clone the existing transaction context
            return Err(sqlx::Error::Configuration(
                "Cannot create nested transaction".into(),
            ));
        }
        drop(guard);

        let txn = self.db_pool.begin().await?;
        Ok(Context {
            invoker: self.invoker.clone(),
            db_pool: self.db_pool.clone(),
            txn: Arc::new(Mutex::new(Some(txn))),
        })
    }
}

impl<'c> sqlx::Executor<'c> for &'c Context {
    type Database = Postgres;

    fn fetch_many<'e, 'q: 'e, E>(
        self,
        query: E,
    ) -> futures::stream::BoxStream<
        'e,
        Result<sqlx::Either<sqlx::postgres::PgQueryResult, sqlx::postgres::PgRow>, sqlx::Error>,
    >
    where
        'c: 'e,
        E: 'q + sqlx::Execute<'q, Self::Database>,
    {
        let txn = self.txn.clone();
        Box::pin(async_stream::stream! {
            let mut guard = txn.lock().await;
            match guard.as_mut() {
                Some(txn) => {
                    let mut stream = txn.fetch_many(query);
                    while let Some(item) = stream.next().await {
                        yield item;
                    }
                }
                None => {
                    let mut stream = self.db_pool.fetch_many(query);
                    while let Some(item) = stream.next().await {
                        yield item;
                    }
                }
            }
        })
    }

    fn fetch_optional<'e, 'q: 'e, E>(
        self,
        query: E,
    ) -> Pin<Box<dyn Future<Output = Result<Option<sqlx::postgres::PgRow>, sqlx::Error>> + Send + 'e>>
    where
        'c: 'e,
        E: 'q + sqlx::Execute<'q, Self::Database>,
    {
        let txn = self.txn.clone();
        let db_pool = self.db_pool.clone();
        Box::pin(async move {
            let mut guard = txn.lock().await;
            match guard.as_mut() {
                Some(txn) => txn.fetch_optional(query).await,
                None => db_pool.fetch_optional(query).await,
            }
        })
    }

    fn prepare_with<'e, 'q: 'e>(
        self,
        sql: &'q str,
        parameters: &'e [sqlx::postgres::PgTypeInfo],
    ) -> Pin<
        Box<dyn Future<Output = Result<sqlx::postgres::PgStatement<'q>, sqlx::Error>> + Send + 'e>,
    >
    where
        'c: 'e,
    {
        let txn = self.txn.clone();
        let db_pool = self.db_pool.clone();
        Box::pin(async move {
            let mut guard = txn.lock().await;
            match guard.as_mut() {
                Some(txn) => txn.prepare_with(sql, parameters).await,
                None => db_pool.prepare_with(sql, parameters).await,
            }
        })
    }

    fn describe<'e, 'q: 'e>(
        self,
        sql: &'q str,
    ) -> Pin<
        Box<dyn Future<Output = Result<sqlx::Describe<Self::Database>, sqlx::Error>> + Send + 'e>,
    >
    where
        'c: 'e,
    {
        let txn = self.txn.clone();
        let db_pool = self.db_pool.clone();
        Box::pin(async move {
            let mut guard = txn.lock().await;
            match guard.as_mut() {
                Some(txn) => txn.describe(sql).await,
                None => db_pool.describe(sql).await,
            }
        })
    }
}

#[async_trait]
impl FromRequestParts<crate::http_server::AppState> for Context {
    type Rejection = ApiError;

    #[tracing::instrument(name = "Context::from_request_parts", skip_all)]
    async fn from_request_parts(
        parts: &mut Parts,
        state: &crate::http_server::AppState,
    ) -> Result<Self, Self::Rejection> {
        // Extract the User from the request
        let user = auth::User::from_request_parts(parts, state).await?;

        // Create the user context and return to the request handler
        Ok(Context::new_user(&user, state.db_pool.clone()))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub enum Invoker {
    /// Represents an invocation by request of a user.
    User { id: Uuid },

    /// Represents a system-level invocation, such as a background job or an internal service call.
    /// The string used should be uniquely grep-able in the codebase.
    System { tag: String },
}

impl Invoker {
    /// Create an Invoker from a User.
    pub fn from_user(user: &auth::User) -> Self {
        Invoker::User { id: user.id }
    }

    /// Create an Invoker from a user ID.
    pub fn from_user_id(id: Uuid) -> Self {
        Invoker::User { id }
    }

    /// Create an Invoker from a system tag.
    pub fn from_system(tag: &'static str) -> Self {
        Invoker::System {
            tag: tag.to_string(),
        }
    }
}
