use crate::context::Context;
use crate::db::id::Id;
use crate::db::result_set::{Filter, ResultSet};
use crate::db::sql::Sql;
use crate::define_id;
use crate::sql;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

define_id!(FooId);

/// Dummy value, for the purposes of this API sketch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Invoker(String);

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct Foo {
    id: FooId, // All records are PK-ed by `id` column.

    name: String, // one "real" column

    created_at: DateTime<Utc>, // standard metadata
    created_by: String,
    updated_at: DateTime<Utc>,
    updated_by: String,
}

/// Filters which can be (optionally) combined to select `Foo` records.
///
/// ```no_run
/// # use api::db::repo::{FooRepo, FooFilter, ReadRepo};
/// # async fn example() -> anyhow::Result<()> {
/// # let pool = sqlx::PgPool::connect("").await?;
/// # let mut conn = pool.acquire().await?;
/// FooRepo::new()
///     .list()
///     .filter(FooFilter::NameIgnoreCase("alice".to_string()))
///     .fetch_all(&mut *conn)
///     .await?;
/// # Ok(())
/// # }
/// ```
pub enum FooFilter {
    Id(FooId),
    Name(String),
    NameIgnoreCase(String),
}

impl Filter for FooFilter {
    type Record = Foo;

    fn as_predicate(&self) -> Sql {
        match self {
            FooFilter::Id(id) => {
                let uuid = id.as_uuid();
                sql!("id = " uuid)
            }
            FooFilter::Name(name) => sql!("name = " name),
            FooFilter::NameIgnoreCase(name) => sql!("LOWER(name) = LOWER(" name ")"),
        }
    }
}

pub trait ReadRepo<T>
where
    T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    type Id: Id + Send;

    /// Start a query to list records of type T.
    fn list(&self) -> ResultSet<T>;

    fn get(&self, id: Self::Id, ctx: &Context) -> impl Future<Output = Result<T>> + Send
    where
        Self: Sync,
    {
        async move { self.list().by_id(id).fetch_one(ctx).await }
    }
}

#[derive(Clone, Copy, Default)]
pub struct FooRepo;

impl FooRepo {
    pub fn new() -> Self {
        Self
    }
}

impl ReadRepo<Foo> for FooRepo {
    type Id = FooId;

    fn list(&self) -> ResultSet<Foo> {
        ResultSet::new(unsafe { Sql::raw("foo") })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires DATABASE_URL to be set"]
    async fn test_foo_repo_list() -> anyhow::Result<()> {
        // Get database connection - need a single connection for temp table
        let database_url = std::env::var("DATABASE_URL")?;
        let pool = sqlx::PgPool::connect(&database_url).await?;
        let mut conn = pool.acquire().await?;

        // Create temporary table
        sqlx::query(
            r#"
            CREATE TEMPORARY TABLE "foo" (
                id UUID PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                created_by TEXT NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                updated_by TEXT NOT NULL
            )
            "#,
        )
        .fetch_all(&mut *conn)
        .await?;

        // Insert three test rows
        let now = Utc::now();
        let id1 = FooId::generate();
        let id2 = FooId::generate();
        let id3 = FooId::generate();

        sqlx::query(
            r#"
            INSERT INTO "foo" (id, name, created_at, created_by, updated_at, updated_by)
            VALUES
                ($1, 'Alice', $2, 'system', $2, 'system'),
                ($3, 'Bob', $2, 'system', $2, 'system'),
                ($4, 'Charlie', $2, 'system', $2, 'system')
            "#,
        )
        .bind(id1.as_uuid())
        .bind(now)
        .bind(id2.as_uuid())
        .bind(id3.as_uuid())
        .fetch_all(&mut *conn)
        .await?;

        // Fetch all records
        let results = FooRepo::new().list().fetch_all(&mut *conn).await?;

        // Verify we got three results
        assert_eq!(results.len(), 3);

        // Verify the names
        let names: Vec<String> = results.iter().map(|f| f.name.clone()).collect();
        assert!(names.contains(&"Alice".to_string()));
        assert!(names.contains(&"Bob".to_string()));
        assert!(names.contains(&"Charlie".to_string()));

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL to be set"]
    async fn test_foo_repo_filter() -> anyhow::Result<()> {
        // Get database connection
        let database_url = std::env::var("DATABASE_URL")?;
        let pool = sqlx::PgPool::connect(&database_url).await?;
        let mut conn = pool.acquire().await?;

        // Create temporary table
        sqlx::query(
            r#"
            CREATE TEMPORARY TABLE "foo" (
                id UUID PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                created_by TEXT NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                updated_by TEXT NOT NULL
            )
            "#,
        )
        .fetch_all(&mut *conn)
        .await?;

        // Insert test rows
        let now = Utc::now();
        let alice_id = FooId::generate();
        let bob_id = FooId::generate();
        let charlie_id = FooId::generate();

        sqlx::query(
            r#"
            INSERT INTO "foo" (id, name, created_at, created_by, updated_at, updated_by)
            VALUES
                ($1, 'Alice', $2, 'system', $2, 'system'),
                ($3, 'Bob', $2, 'system', $2, 'system'),
                ($4, 'Charlie', $2, 'system', $2, 'system')
            "#,
        )
        .bind(alice_id.as_uuid())
        .bind(now)
        .bind(bob_id.as_uuid())
        .bind(charlie_id.as_uuid())
        .fetch_all(&mut *conn)
        .await?;

        // Test: Filter by exact name
        let results = FooRepo::new()
            .list()
            .filter(FooFilter::Name("Alice".to_string()))
            .fetch_all(&mut *conn)
            .await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Alice");

        // Test: Filter by ID
        let results = FooRepo::new()
            .list()
            .filter(FooFilter::Id(bob_id))
            .fetch_all(&mut *conn)
            .await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Bob");

        // Test: Filter by name (case-insensitive)
        let results = FooRepo::new()
            .list()
            .filter(FooFilter::NameIgnoreCase("ALICE".to_string()))
            .fetch_all(&mut *conn)
            .await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Alice");

        // Test: Multiple filters (should combine with AND)
        let results = FooRepo::new()
            .list()
            .filter(FooFilter::Id(alice_id))
            .filter(FooFilter::Name("Alice".to_string()))
            .fetch_all(&mut *conn)
            .await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Alice");

        // Test: Multiple filters with no match
        let results = FooRepo::new()
            .list()
            .filter(FooFilter::Id(alice_id))
            .filter(FooFilter::Name("Bob".to_string()))
            .fetch_all(&mut *conn)
            .await?;
        assert_eq!(results.len(), 0);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL to be set"]
    async fn test_foo_repo_limit() -> anyhow::Result<()> {
        // Get database connection
        let database_url = std::env::var("DATABASE_URL")?;
        let pool = sqlx::PgPool::connect(&database_url).await?;
        let mut conn = pool.acquire().await?;

        // Create temporary table
        sqlx::query(
            r#"
            CREATE TEMPORARY TABLE "foo" (
                id UUID PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                created_by TEXT NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                updated_by TEXT NOT NULL
            )
            "#,
        )
        .fetch_all(&mut *conn)
        .await?;

        // Insert test rows
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO "foo" (id, name, created_at, created_by, updated_at, updated_by)
            VALUES
                ($1, 'Alice', $2, 'system', $2, 'system'),
                ($3, 'Bob', $2, 'system', $2, 'system'),
                ($4, 'Charlie', $2, 'system', $2, 'system'),
                ($5, 'David', $2, 'system', $2, 'system'),
                ($6, 'Eve', $2, 'system', $2, 'system')
            "#,
        )
        .bind(FooId::generate().as_uuid())
        .bind(now)
        .bind(FooId::generate().as_uuid())
        .bind(FooId::generate().as_uuid())
        .bind(FooId::generate().as_uuid())
        .bind(FooId::generate().as_uuid())
        .fetch_all(&mut *conn)
        .await?;

        // Test: Limit to 2 results
        let results = FooRepo::new().list().limit(2).fetch_all(&mut *conn).await?;
        assert_eq!(results.len(), 2);

        // Test: Limit to 0 results
        let results = FooRepo::new().list().limit(0).fetch_all(&mut *conn).await?;
        assert_eq!(results.len(), 0);

        // Test: Limit with filter
        let results = FooRepo::new()
            .list()
            .filter(FooFilter::NameIgnoreCase("EVE".to_string()))
            .limit(1)
            .fetch_all(&mut *conn)
            .await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Eve");

        // Test: No limit returns all
        let results = FooRepo::new().list().fetch_all(&mut *conn).await?;
        assert_eq!(results.len(), 5);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL to be set"]
    async fn test_foo_repo_get_by_id() -> anyhow::Result<()> {
        // Get database connection - need a single connection for temp table
        let database_url = std::env::var("DATABASE_URL")?;
        let pool = sqlx::PgPool::connect(&database_url).await?;
        let mut conn = pool.acquire().await?;

        // Create temporary table
        sqlx::query(
            r#"
            CREATE TEMPORARY TABLE "foo" (
                id UUID PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                created_by TEXT NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                updated_by TEXT NOT NULL
            )
            "#,
        )
        .execute(&mut *conn)
        .await?;

        // Insert test rows
        let now = Utc::now();
        let alice_id = FooId::generate();
        let bob_id = FooId::generate();

        sqlx::query(
            r#"
            INSERT INTO "foo" (id, name, created_at, created_by, updated_at, updated_by)
            VALUES
                ($1, 'Alice', $2, 'system', $2, 'system'),
                ($3, 'Bob', $2, 'system', $2, 'system')
            "#,
        )
        .bind(alice_id.as_uuid())
        .bind(now)
        .bind(bob_id.as_uuid())
        .execute(&mut *conn)
        .await?;

        // Test: Get by ID returns the correct record (using by_id directly since we can't use Context with temp tables)
        let result = FooRepo::new()
            .list()
            .by_id(alice_id)
            .fetch_one(&mut *conn)
            .await?;
        assert_eq!(result.id, alice_id);
        assert_eq!(result.name, "Alice");

        // Test: Get another record by ID
        let result = FooRepo::new()
            .list()
            .by_id(bob_id)
            .fetch_one(&mut *conn)
            .await?;
        assert_eq!(result.id, bob_id);
        assert_eq!(result.name, "Bob");

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL to be set"]
    async fn test_foo_repo_get_nonexistent() -> anyhow::Result<()> {
        // Get database connection - need a single connection for temp table
        let database_url = std::env::var("DATABASE_URL")?;
        let pool = sqlx::PgPool::connect(&database_url).await?;
        let mut conn = pool.acquire().await?;

        // Create temporary table
        sqlx::query(
            r#"
            CREATE TEMPORARY TABLE "foo" (
                id UUID PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                created_by TEXT NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                updated_by TEXT NOT NULL
            )
            "#,
        )
        .execute(&mut *conn)
        .await?;

        // Test: Get nonexistent ID returns error (using by_id directly since we can't use Context with temp tables)
        let nonexistent_id = FooId::generate();
        let result = FooRepo::new()
            .list()
            .by_id(nonexistent_id)
            .fetch_one(&mut *conn)
            .await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("No results found"),
            "Expected error message to contain 'No results found', got: {}",
            err_msg
        );

        Ok(())
    }
}
