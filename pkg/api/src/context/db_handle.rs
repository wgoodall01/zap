use futures::StreamExt;
use sqlx::{Postgres, Transaction};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub enum DbHandle {
    Pool(sqlx::Pool<sqlx::Postgres>),
    Txn(Arc<Mutex<Transaction<'static, Postgres>>>),
}

impl<'c> sqlx::Executor<'c> for &'c DbHandle {
    type Database = Postgres;

    fn fetch_many<'e, 'q: 'e, E: 'q>(
        self,
        query: E,
    ) -> futures::stream::BoxStream<'e, Result<sqlx::Either<sqlx::postgres::PgQueryResult, sqlx::postgres::PgRow>, sqlx::Error>>
    where
        'c: 'e,
        E: sqlx::Execute<'q, Self::Database>,
    {
        match self {
            DbHandle::Pool(pool) => pool.fetch_many(query),
            DbHandle::Txn(txn) => {
                let txn = txn.clone();
                Box::pin(async_stream::stream! {
                    let mut guard = txn.lock().await;
                    let mut stream = guard.fetch_many(query);
                    while let Some(item) = stream.next().await {
                        yield item;
                    }
                })
            }
        }
    }

    fn fetch_optional<'e, 'q: 'e, E: 'q>(
        self,
        query: E,
    ) -> Pin<Box<dyn Future<Output = Result<Option<sqlx::postgres::PgRow>, sqlx::Error>> + Send + 'e>>
    where
        'c: 'e,
        E: sqlx::Execute<'q, Self::Database>,
    {
        match self {
            DbHandle::Pool(pool) => pool.fetch_optional(query),
            DbHandle::Txn(txn) => {
                let txn = txn.clone();
                Box::pin(async move {
                    let mut guard = txn.lock().await;
                    guard.fetch_optional(query).await
                })
            }
        }
    }

    fn prepare_with<'e, 'q: 'e>(
        self,
        sql: &'q str,
        parameters: &'e [sqlx::postgres::PgTypeInfo],
    ) -> Pin<Box<dyn Future<Output = Result<sqlx::postgres::PgStatement<'q>, sqlx::Error>> + Send + 'e>>
    where
        'c: 'e,
    {
        match self {
            DbHandle::Pool(pool) => pool.prepare_with(sql, parameters),
            DbHandle::Txn(txn) => {
                let txn = txn.clone();
                Box::pin(async move {
                    let mut guard = txn.lock().await;
                    guard.prepare_with(sql, parameters).await
                })
            }
        }
    }

    fn describe<'e, 'q: 'e>(
        self,
        sql: &'q str,
    ) -> Pin<Box<dyn Future<Output = Result<sqlx::Describe<Self::Database>, sqlx::Error>> + Send + 'e>>
    where
        'c: 'e,
    {
        match self {
            DbHandle::Pool(pool) => pool.describe(sql),
            DbHandle::Txn(txn) => {
                let txn = txn.clone();
                Box::pin(async move {
                    let mut guard = txn.lock().await;
                    guard.describe(sql).await
                })
            }
        }
    }
}