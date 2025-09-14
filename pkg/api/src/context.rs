use crate::auth;
use futures::StreamExt;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::State;
use rocket_okapi::okapi::openapi3::{SecurityScheme, SecuritySchemeData};
use rocket_okapi::okapi::Map;
use rocket_okapi::request::{OpenApiFromRequest, RequestHeaderInput};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Transaction};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
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
            invoker: Invoker::from_user(&user),
            db_pool,
            txn: Arc::new(Mutex::new(None)),
        }
    }

    /// Commit the current transaction, if it exists.
    pub async fn commit(&self) -> Result<(), sqlx::Error> {
        let mut guard = self.txn.lock().await;
        if let Some(txn) = guard.take() {
            txn.commit().await
        } else {
            Ok(())
        }
    }

    /// Create a child context which runs in a single database transaction.
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

    fn fetch_many<'e, 'q: 'e, E: 'q>(
        self,
        query: E,
    ) -> futures::stream::BoxStream<
        'e,
        Result<sqlx::Either<sqlx::postgres::PgQueryResult, sqlx::postgres::PgRow>, sqlx::Error>,
    >
    where
        'c: 'e,
        E: sqlx::Execute<'q, Self::Database>,
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

    fn fetch_optional<'e, 'q: 'e, E: 'q>(
        self,
        query: E,
    ) -> Pin<Box<dyn Future<Output = Result<Option<sqlx::postgres::PgRow>, sqlx::Error>> + Send + 'e>>
    where
        'c: 'e,
        E: sqlx::Execute<'q, Self::Database>,
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

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Context {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Get the database pool from Rocket state
        let db_pool = request
            .guard::<&State<sqlx::Pool<sqlx::Postgres>>>()
            .await
            .expect("Database pool not found in Rocket state");

        // Extract the User from the request.
        let user = match request.guard::<auth::User>().await {
            Outcome::Success(user) => user,
            Outcome::Error((status, _)) => return Outcome::Error((status, ())),
            Outcome::Forward(_) => return Outcome::Error((rocket::http::Status::Unauthorized, ())),
        };

        // Create the user context and return to the request handler.
        Outcome::Success(Context::new_user(&user, db_pool.inner().clone()))
    }
}

impl<'r> OpenApiFromRequest<'r> for Context {
    fn from_request_input(
        _gen: &mut rocket_okapi::r#gen::OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> rocket_okapi::Result<RequestHeaderInput> {
        let security_scheme = SecurityScheme {
            data: SecuritySchemeData::Http {
                scheme: "bearer".to_owned(),
                bearer_format: Some("Telegram raw_init_data (with signature)".to_owned()),
            },
            description: Some("Telegram MiniApp init-data token".to_owned()),
            extensions: Map::new(),
        };

        let mut security_req = Map::new();
        security_req.insert("bearer".to_owned(), vec![]);

        Ok(RequestHeaderInput::Security(
            "bearer".to_owned(),
            security_scheme,
            security_req,
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
