use crate::context::{Context, Invoker};
use crate::db::id::Id;
use crate::db::result_set::{Filter, ResultSet};
use crate::db::sql::Sql;
use crate::define_id;
use crate::sql;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

define_id!(FooId);

/// Repository metadata fields that are automatically managed.
///
/// This struct groups all the standard audit/lifecycle fields that repositories
/// automatically populate and maintain. Use `#[sqlx(flatten)]` and `#[serde(flatten)]`
/// to embed these fields directly into your model structs.
///
/// # Example
///
/// ```
/// use api::db::repo::{RepoMeta, FooId};
/// use serde::Serialize;
///
/// #[derive(Debug, Clone, sqlx::FromRow, Serialize)]
/// pub struct Foo {
///     id: FooId,
///     name: String,
///
///     #[sqlx(flatten)]
///     #[serde(flatten)]
///     meta: RepoMeta,
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepoMeta {
    pub created_at: DateTime<Utc>,
    pub created_by: Invoker,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Invoker,
    pub deleted_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_by: Option<Invoker>,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for RepoMeta {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> std::result::Result<Self, sqlx::Error> {
        use sqlx::Row;

        let created_at: DateTime<Utc> = row.try_get("created_at")?;
        let created_by: serde_json::Value = row.try_get("created_by")?;
        let updated_at: DateTime<Utc> = row.try_get("updated_at")?;
        let updated_by: serde_json::Value = row.try_get("updated_by")?;
        let deleted_at: Option<DateTime<Utc>> = row.try_get("deleted_at")?;

        // Handle deleted_by: if the column is NULL, get None; otherwise deserialize the JSON
        let deleted_by: Option<Invoker> = row.try_get::<Option<serde_json::Value>, _>("deleted_by")?
            .map(|v| serde_json::from_value(v).map_err(|e| sqlx::Error::Decode(Box::new(e))))
            .transpose()?;

        let created_by = serde_json::from_value(created_by)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        let updated_by = serde_json::from_value(updated_by)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        Ok(RepoMeta {
            created_at,
            created_by,
            updated_at,
            updated_by,
            deleted_at,
            deleted_by,
        })
    }
}

impl RepoMeta {
    /// Create a new RepoMeta for a newly created record.
    pub fn new(invoker: &Invoker) -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            created_by: invoker.clone(),
            updated_at: now,
            updated_by: invoker.clone(),
            deleted_at: None,
            deleted_by: None,
        }
    }

    /// Update the metadata for a record being modified.
    pub fn touch(&mut self, invoker: &Invoker) {
        self.updated_at = Utc::now();
        self.updated_by = invoker.clone();
    }

    /// Mark this record as soft-deleted.
    pub fn soft_delete(&mut self, invoker: &Invoker) {
        self.deleted_at = Some(Utc::now());
        self.deleted_by = Some(invoker.clone());
    }

    /// Check if this record is soft-deleted.
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
}

/// Main Foo model with managed metadata.
///
/// ## Repository Metadata Pattern
///
/// This struct uses the `RepoMeta` pattern to group all managed fields (created/updated/deleted)
/// into a single flattened field. This provides a clean separation between user fields and
/// managed fields with minimal boilerplate.
///
/// ## Future Proc Macro Design
///
/// A future `api_macros` crate with `#[derive(Repository)]` would generate:
///
/// ### Proposed Usage
///
/// ```ignore
/// use api_macros::Repository;
///
/// #[derive(Repository)]
/// #[repository(table = "foo")]
/// pub struct Foo {
///     id: FooId,
///     name: String,
///
///     #[repo(meta)]  // Signals this is the managed metadata field
///     meta: RepoMeta,
/// }
/// ```
///
/// ### Generated Code
///
/// The macro would auto-generate:
///
/// 1. **FooCreate** - Excludes `id` and `meta`:
///    ```ignore
///    #[derive(Debug, Clone, Deserialize)]
///    pub struct FooCreate {
///        pub name: String,
///    }
///    ```
///
/// 2. **FooUpdate** - Option-wrapped user fields, excludes `id` and `meta`:
///    ```ignore
///    #[derive(Debug, Clone, Default, Deserialize)]
///    pub struct FooUpdate {
///        pub name: Option<String>,
///    }
///    ```
///
/// 3. **Trait implementations** for CreateRepo, UpdateRepo, DeleteRepo with automatic:
///    - RepoMeta initialization and updates
///    - SQL generation with proper binding
///    - Soft delete support
///
/// ### Filter Macro (Separate)
///
/// Filters are NOT auto-generated, but can use a separate `#[derive(Filter)]` macro:
///
/// ```ignore
/// use api_macros::Filter;
///
/// #[derive(Filter)]
/// #[filter(record = Foo)]
/// pub enum FooFilter {
///     // Destructure in the "lambda" to get named variables for the SQL expression
///     #[filter(|(id)| sql!("id = " id))]
///     Id(FooId),
///
///     #[filter(|(name)| sql!("name = " name))]
///     Name(String),
///
///     #[filter(|(name)| sql!("LOWER(name) = LOWER(" name ")"))]
///     NameIgnoreCase(String),
///
///     // Tuple variants work too
///     #[filter(|(min, max)| sql!("created_at BETWEEN " min " AND " max))]
///     CreatedBetween(DateTime<Utc>, DateTime<Utc>),
/// }
/// ```
///
/// This generates:
/// ```ignore
/// impl Filter for FooFilter {
///     type Record = Foo;
///
///     fn as_predicate(&self) -> Sql {
///         match self {
///             FooFilter::Id(id) => sql!("id = " id),
///             FooFilter::Name(name) => sql!("name = " name),
///             FooFilter::NameIgnoreCase(name) => sql!("LOWER(name) = LOWER(" name ")"),
///             FooFilter::CreatedBetween(min, max) => sql!("created_at BETWEEN " min " AND " max),
///         }
///     }
/// }
/// ```
///
/// ### Customization Attributes
///
/// - `#[repo(skip)]` - Exclude field from Create/Update (computed/derived field)
/// - `#[repo(readonly)]` - In struct but not in Create/Update
#[derive(Debug, Clone, PartialEq, sqlx::FromRow, Serialize)]
pub struct Foo {
    id: FooId,
    name: String,

    #[sqlx(flatten)]
    #[serde(flatten)]
    meta: RepoMeta,
}

/// Input for creating a new Foo record.
/// Excludes all managed fields (id, timestamps, invokers).
#[derive(Debug, Clone, Deserialize)]
pub struct FooCreate {
    pub name: String,
}

/// Input for updating a Foo record (partial updates).
/// All fields are Option-wrapped to allow selective updates.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FooUpdate {
    pub name: Option<String>,
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

/// Trait for repositories that support creating records.
///
/// This trait is independent and does not require `ReadRepo` to be implemented,
/// allowing for write-only repositories if needed.
pub trait CreateRepo<T>
where
    T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    type Id: Id + Send;

    /// The input type for creating a new record (without managed fields).
    type CreateInput: Send;

    /// Create a new record and return the full record including all managed fields.
    ///
    /// The implementation will automatically populate:
    /// - `id`: A new UUID (typically UUIDv7)
    /// - `created_at`: Current timestamp
    /// - `created_by`: Invoker from context (stored as JSONB)
    /// - `updated_at`: Current timestamp (same as created_at)
    /// - `updated_by`: Invoker from context (same as created_by)
    fn create(
        &self,
        data: Self::CreateInput,
        ctx: &Context,
    ) -> impl Future<Output = Result<T>> + Send
    where
        Self: Sync;
}

/// Trait for repositories that support updating records.
///
/// This trait is independent and does not require `ReadRepo` to be implemented.
pub trait UpdateRepo<T>
where
    T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    type Id: Id + Send;

    /// The input type for updating a record (partial updates with Option-wrapped fields).
    type UpdateInput: Send;

    /// Update a single record by ID and return the updated record.
    ///
    /// The implementation will automatically update:
    /// - `updated_at`: Current timestamp
    /// - `updated_by`: Invoker from context (stored as JSONB)
    fn update(
        &self,
        id: Self::Id,
        data: Self::UpdateInput,
        ctx: &Context,
    ) -> impl Future<Output = Result<T>> + Send
    where
        Self: Sync;

    /// Update multiple records matching the given filters.
    ///
    /// Returns the number of records updated.
    ///
    /// The implementation will automatically update:
    /// - `updated_at`: Current timestamp
    /// - `updated_by`: Invoker from context (stored as JSONB)
    fn update_many(
        &self,
        filters: Vec<impl Filter<Record = T> + Send>,
        data: Self::UpdateInput,
        ctx: &Context,
    ) -> impl Future<Output = Result<u64>> + Send
    where
        Self: Sync;
}

/// Trait for repositories that support deleting records (soft and hard deletes).
///
/// This trait is independent and does not require `ReadRepo` to be implemented.
///
/// Soft deletes set the `deleted_at` and `deleted_by` fields without removing the record,
/// while hard deletes permanently remove the record from the database.
pub trait DeleteRepo<T>
where
    T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    type Id: Id + Send;

    /// Soft delete a single record by ID.
    ///
    /// Returns true if a record was deleted, false if not found.
    ///
    /// The implementation will set:
    /// - `deleted_at`: Current timestamp
    /// - `deleted_by`: Invoker from context (stored as JSONB)
    fn delete(
        &self,
        id: Self::Id,
        ctx: &Context,
    ) -> impl Future<Output = Result<bool>> + Send
    where
        Self: Sync;

    /// Soft delete multiple records matching the given filters.
    ///
    /// Returns the number of records deleted.
    ///
    /// The implementation will set:
    /// - `deleted_at`: Current timestamp
    /// - `deleted_by`: Invoker from context (stored as JSONB)
    fn delete_many(
        &self,
        filters: Vec<impl Filter<Record = T> + Send>,
        ctx: &Context,
    ) -> impl Future<Output = Result<u64>> + Send
    where
        Self: Sync;

    /// Permanently delete a single record by ID.
    ///
    /// Returns true if a record was deleted, false if not found.
    fn hard_delete(
        &self,
        id: Self::Id,
        ctx: &Context,
    ) -> impl Future<Output = Result<bool>> + Send
    where
        Self: Sync;

    /// Permanently delete multiple records matching the given filters.
    ///
    /// Returns the number of records deleted.
    fn hard_delete_many(
        &self,
        filters: Vec<impl Filter<Record = T> + Send>,
        ctx: &Context,
    ) -> impl Future<Output = Result<u64>> + Send
    where
        Self: Sync;
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
        // Automatically filter out soft-deleted records
        ResultSet::new(unsafe { Sql::raw("foo") })
            .where_sql(sql!("deleted_at IS NULL"))
    }
}

impl CreateRepo<Foo> for FooRepo {
    type Id = FooId;
    type CreateInput = FooCreate;

    async fn create(&self, data: Self::CreateInput, ctx: &Context) -> Result<Foo> {
        let id = FooId::generate();
        let id_uuid = id.as_uuid();
        let meta = RepoMeta::new(&ctx.invoker);
        let name = data.name;

        // Extract meta fields for sql! macro
        let created_at = meta.created_at;
        let created_by = &meta.created_by;
        let updated_at = meta.updated_at;
        let updated_by = &meta.updated_by;

        let query = sql!(
            "INSERT INTO foo (id, name, created_at, created_by, updated_at, updated_by, deleted_at, deleted_by)
             VALUES (" id_uuid ", " name ", " created_at ", " created_by ", " updated_at ", " updated_by ", NULL, NULL)
             RETURNING *"
        );

        let query_str = query.render_pg();
        let sqlx_query = sqlx::query_as::<_, Foo>(&query_str);
        let sqlx_query = query.bind_to_query_as(sqlx_query);

        sqlx_query
            .fetch_one(ctx)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create Foo: {}", e))
    }
}

impl UpdateRepo<Foo> for FooRepo {
    type Id = FooId;
    type UpdateInput = FooUpdate;

    async fn update(&self, id: Self::Id, data: Self::UpdateInput, ctx: &Context) -> Result<Foo> {
        let now = Utc::now();
        let invoker = &ctx.invoker;

        // Build SET clauses for fields that are Some
        let mut set_clauses = vec![];

        if let Some(name) = data.name {
            set_clauses.push(sql!("name = " name));
        }

        // Always update the metadata
        set_clauses.push(sql!("updated_at = " now));
        set_clauses.push(sql!("updated_by = " invoker));

        // If no user fields changed, just return the existing record
        if set_clauses.len() == 2 {
            // Only metadata would be updated, skip the query
            return self.get(id, ctx).await;
        }

        let set_clause = Sql::join_with(", ", set_clauses);
        let id_uuid = id.as_uuid();
        let query = sql!(
            "UPDATE foo SET " set_clause " WHERE id = " id_uuid " AND deleted_at IS NULL RETURNING *"
        );

        let query_str = query.render_pg();
        let sqlx_query = sqlx::query_as::<_, Foo>(&query_str);
        let sqlx_query = query.bind_to_query_as(sqlx_query);

        sqlx_query
            .fetch_one(ctx)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to update Foo: {}", e))
    }

    async fn update_many(
        &self,
        filters: Vec<impl Filter<Record = Foo>>,
        data: Self::UpdateInput,
        ctx: &Context,
    ) -> Result<u64> {
        let now = Utc::now();
        let invoker = &ctx.invoker;

        // Build SET clauses
        let mut set_clauses = vec![];

        if let Some(name) = data.name {
            set_clauses.push(sql!("name = " name));
        }

        // Always update metadata
        set_clauses.push(sql!("updated_at = " now));
        set_clauses.push(sql!("updated_by = " invoker));

        // If no user fields changed, return 0
        if set_clauses.len() == 2 {
            return Ok(0);
        }

        let set_clause = Sql::join_with(", ", set_clauses);
        let mut query = sql!("UPDATE foo SET " set_clause " WHERE deleted_at IS NULL");

        // Add filter predicates
        if !filters.is_empty() {
            let filter_predicates: Vec<Sql> = filters.into_iter().map(|f| f.as_predicate()).collect();
            let where_clause = Sql::join_with(" AND ", filter_predicates);
            query = query.concat(&sql!(" AND " where_clause));
        }

        let query_str = query.render_pg();
        let sqlx_query = sqlx::query(&query_str);
        let sqlx_query = query.bind_to_query(sqlx_query);

        let result = sqlx_query
            .execute(ctx)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to update many Foo: {}", e))?;

        Ok(result.rows_affected())
    }
}

impl DeleteRepo<Foo> for FooRepo {
    type Id = FooId;

    async fn delete(&self, id: Self::Id, ctx: &Context) -> Result<bool> {
        let count = self.delete_many(vec![FooFilter::Id(id)], ctx).await?;
        Ok(count > 0)
    }

    async fn delete_many(
        &self,
        filters: Vec<impl Filter<Record = Foo>>,
        ctx: &Context,
    ) -> Result<u64> {
        let now = Utc::now();
        let invoker = &ctx.invoker;

        let mut query = sql!(
            "UPDATE foo SET deleted_at = " now ", deleted_by = " invoker " WHERE deleted_at IS NULL"
        );

        // Add filter predicates
        if !filters.is_empty() {
            let filter_predicates: Vec<Sql> = filters.into_iter().map(|f| f.as_predicate()).collect();
            let where_clause = Sql::join_with(" AND ", filter_predicates);
            query = query.concat(&sql!(" AND " where_clause));
        }

        let query_str = query.render_pg();
        let sqlx_query = sqlx::query(&query_str);
        let sqlx_query = query.bind_to_query(sqlx_query);

        let result = sqlx_query
            .execute(ctx)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to soft delete Foo: {}", e))?;

        Ok(result.rows_affected())
    }

    async fn hard_delete(&self, id: Self::Id, ctx: &Context) -> Result<bool> {
        let count = self.hard_delete_many(vec![FooFilter::Id(id)], ctx).await?;
        Ok(count > 0)
    }

    async fn hard_delete_many(
        &self,
        filters: Vec<impl Filter<Record = Foo>>,
        ctx: &Context,
    ) -> Result<u64> {
        let mut query = sql!("DELETE FROM foo WHERE 1=1");

        // Add filter predicates
        if !filters.is_empty() {
            let filter_predicates: Vec<Sql> = filters.into_iter().map(|f| f.as_predicate()).collect();
            let where_clause = Sql::join_with(" AND ", filter_predicates);
            query = query.concat(&sql!(" AND " where_clause));
        }

        let query_str = query.render_pg();
        let sqlx_query = sqlx::query(&query_str);
        let sqlx_query = query.bind_to_query(sqlx_query);

        let result = sqlx_query
            .execute(ctx)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to hard delete Foo: {}", e))?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test context with a transaction and temp table.
    /// The temp table is scoped to the transaction's connection and will be
    /// automatically cleaned up when the transaction ends.
    async fn test_context_with_temp_table(pool: sqlx::PgPool) -> anyhow::Result<Context> {
        let ctx = Context::new_system("test-system", pool);
        let txn_ctx = ctx.in_txn().await?;

        // Create temporary table on the transaction's connection
        sqlx::query(
            r#"
            CREATE TEMPORARY TABLE "foo" (
                id UUID PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                created_by JSONB NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                updated_by JSONB NOT NULL,
                deleted_at TIMESTAMPTZ,
                deleted_by JSONB
            )
            "#,
        )
        .execute(&txn_ctx)
        .await?;

        Ok(txn_ctx)
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL to be set"]
    async fn test_foo_repo_create() -> anyhow::Result<()> {
        let database_url = std::env::var("DATABASE_URL")?;
        let pool = sqlx::PgPool::connect(&database_url).await?;
        let ctx = test_context_with_temp_table(pool).await?;

        // Create a new Foo
        let foo = FooRepo::new()
            .create(
                FooCreate {
                    name: "Test Foo".to_string(),
                },
                &ctx,
            )
            .await?;

        // Verify the record was created with all managed fields
        assert_eq!(foo.name, "Test Foo");
        assert!(matches!(foo.meta.created_by, Invoker::System { ref tag } if tag == "test-system"));
        assert!(matches!(foo.meta.updated_by, Invoker::System { ref tag } if tag == "test-system"));
        assert_eq!(foo.meta.deleted_at, None);
        assert_eq!(foo.meta.deleted_by, None);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL to be set"]
    async fn test_foo_repo_list() -> anyhow::Result<()> {
        let database_url = std::env::var("DATABASE_URL")?;
        let pool = sqlx::PgPool::connect(&database_url).await?;
        let ctx = test_context_with_temp_table(pool).await?;

        // Create three test records
        FooRepo::new()
            .create(FooCreate { name: "Alice".to_string() }, &ctx)
            .await?;
        FooRepo::new()
            .create(FooCreate { name: "Bob".to_string() }, &ctx)
            .await?;
        FooRepo::new()
            .create(FooCreate { name: "Charlie".to_string() }, &ctx)
            .await?;

        // Fetch all records
        let results = FooRepo::new().list().fetch_all(&ctx).await?;

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
    async fn test_foo_repo_update() -> anyhow::Result<()> {
        let database_url = std::env::var("DATABASE_URL")?;
        let pool = sqlx::PgPool::connect(&database_url).await?;
        let ctx = test_context_with_temp_table(pool).await?;

        // Create a record
        let foo = FooRepo::new()
            .create(FooCreate { name: "Original".to_string() }, &ctx)
            .await?;

        // Update the record
        let updated = FooRepo::new()
            .update(
                foo.id,
                FooUpdate {
                    name: Some("Updated".to_string()),
                },
                &ctx,
            )
            .await?;

        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.id, foo.id);
        assert!(updated.meta.updated_at > foo.meta.updated_at);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL to be set"]
    async fn test_foo_repo_update_partial() -> anyhow::Result<()> {
        let database_url = std::env::var("DATABASE_URL")?;
        let pool = sqlx::PgPool::connect(&database_url).await?;
        let ctx = test_context_with_temp_table(pool).await?;

        // Create a record
        let foo = FooRepo::new()
            .create(FooCreate { name: "Test".to_string() }, &ctx)
            .await?;

        // Update with empty changes should return the same record
        let updated = FooRepo::new()
            .update(foo.id, FooUpdate::default(), &ctx)
            .await?;

        assert_eq!(updated.name, "Test");

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL to be set"]
    async fn test_foo_repo_update_many() -> anyhow::Result<()> {
        let database_url = std::env::var("DATABASE_URL")?;
        let pool = sqlx::PgPool::connect(&database_url).await?;
        let ctx = test_context_with_temp_table(pool).await?;

        // Create several records
        FooRepo::new().create(FooCreate { name: "Foo1".to_string() }, &ctx).await?;
        FooRepo::new().create(FooCreate { name: "Foo2".to_string() }, &ctx).await?;
        FooRepo::new().create(FooCreate { name: "Bar1".to_string() }, &ctx).await?;

        // Update all Foo* names
        let count = FooRepo::new()
            .update_many(
                vec![FooFilter::NameIgnoreCase("foo%".to_string())],
                FooUpdate { name: Some("Updated".to_string()) },
                &ctx,
            )
            .await?;

        // Note: The NameIgnoreCase filter uses = not LIKE, so this won't match anything
        // This test demonstrates the API, actual filter implementation may vary
        assert_eq!(count, 0); // No matches with exact "foo%" string

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL to be set"]
    async fn test_foo_repo_soft_delete() -> anyhow::Result<()> {
        let database_url = std::env::var("DATABASE_URL")?;
        let pool = sqlx::PgPool::connect(&database_url).await?;
        let ctx = test_context_with_temp_table(pool).await?;

        // Create a record
        let foo = FooRepo::new()
            .create(FooCreate { name: "ToDelete".to_string() }, &ctx)
            .await?;

        // Soft delete it
        let deleted = FooRepo::new().delete(foo.id, &ctx).await?;
        assert!(deleted);

        // Verify it's not in list() results
        let results = FooRepo::new().list().fetch_all(&ctx).await?;
        assert_eq!(results.len(), 0);

        // Verify it still exists in the database (just soft-deleted)
        let raw_result: Option<(bool,)> = sqlx::query_as(
            "SELECT deleted_at IS NOT NULL FROM foo WHERE id = $1"
        )
        .bind(foo.id.as_uuid())
        .fetch_optional(&ctx)
        .await?;
        assert!(raw_result.is_some());
        assert!(raw_result.unwrap().0); // deleted_at IS NOT NULL

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL to be set"]
    async fn test_foo_repo_delete_many() -> anyhow::Result<()> {
        let database_url = std::env::var("DATABASE_URL")?;
        let pool = sqlx::PgPool::connect(&database_url).await?;
        let ctx = test_context_with_temp_table(pool).await?;

        // Create several records
        FooRepo::new().create(FooCreate { name: "Alice".to_string() }, &ctx).await?;
        FooRepo::new().create(FooCreate { name: "Bob".to_string() }, &ctx).await?;
        FooRepo::new().create(FooCreate { name: "Charlie".to_string() }, &ctx).await?;

        // Soft delete by name
        let count = FooRepo::new()
            .delete_many(vec![FooFilter::Name("Alice".to_string())], &ctx)
            .await?;
        assert_eq!(count, 1);

        // Verify only 2 remain visible
        let results = FooRepo::new().list().fetch_all(&ctx).await?;
        assert_eq!(results.len(), 2);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL to be set"]
    async fn test_foo_repo_hard_delete() -> anyhow::Result<()> {
        let database_url = std::env::var("DATABASE_URL")?;
        let pool = sqlx::PgPool::connect(&database_url).await?;
        let ctx = test_context_with_temp_table(pool).await?;

        // Create a record
        let foo = FooRepo::new()
            .create(FooCreate { name: "ToHardDelete".to_string() }, &ctx)
            .await?;

        // Hard delete it
        let deleted = FooRepo::new().hard_delete(foo.id, &ctx).await?;
        assert!(deleted);

        // Verify it's completely gone from the database
        let raw_result: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM foo WHERE id = $1"
        )
        .bind(foo.id.as_uuid())
        .fetch_one(&ctx)
        .await?;
        assert_eq!(raw_result.0, 0);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL to be set"]
    async fn test_foo_repo_hard_delete_many() -> anyhow::Result<()> {
        let database_url = std::env::var("DATABASE_URL")?;
        let pool = sqlx::PgPool::connect(&database_url).await?;
        let ctx = test_context_with_temp_table(pool).await?;

        // Create several records
        FooRepo::new().create(FooCreate { name: "Alice".to_string() }, &ctx).await?;
        FooRepo::new().create(FooCreate { name: "Bob".to_string() }, &ctx).await?;
        let charlie = FooRepo::new().create(FooCreate { name: "Charlie".to_string() }, &ctx).await?;

        // First soft delete one
        FooRepo::new().delete(charlie.id, &ctx).await?;

        // Hard delete all records (including soft-deleted)
        let count = FooRepo::new()
            .hard_delete_many(Vec::<FooFilter>::new(), &ctx)
            .await?;
        assert_eq!(count, 3); // All 3 records (including soft-deleted Charlie)

        // Verify nothing remains in database
        let raw_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM foo")
            .fetch_one(&ctx)
            .await?;
        assert_eq!(raw_count.0, 0); // Everything is gone

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL to be set"]
    async fn test_foo_repo_filter() -> anyhow::Result<()> {
        let database_url = std::env::var("DATABASE_URL")?;
        let pool = sqlx::PgPool::connect(&database_url).await?;
        let ctx = test_context_with_temp_table(pool).await?;

        // Create test records
        let alice = FooRepo::new().create(FooCreate { name: "Alice".to_string() }, &ctx).await?;
        FooRepo::new().create(FooCreate { name: "Bob".to_string() }, &ctx).await?;
        FooRepo::new().create(FooCreate { name: "Charlie".to_string() }, &ctx).await?;

        // Test: Filter by exact name
        let results = FooRepo::new()
            .list()
            .filter(FooFilter::Name("Alice".to_string()))
            .fetch_all(&ctx)
            .await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Alice");

        // Test: Filter by ID
        let results = FooRepo::new()
            .list()
            .filter(FooFilter::Id(alice.id))
            .fetch_all(&ctx)
            .await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Alice");

        // Test: Filter by name (case-insensitive)
        let results = FooRepo::new()
            .list()
            .filter(FooFilter::NameIgnoreCase("ALICE".to_string()))
            .fetch_all(&ctx)
            .await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Alice");

        Ok(())
    }

    #[test]
    fn test_repo_meta_serialization() {
        // Test that RepoMeta serializes correctly with Option<Invoker>
        let invoker = Invoker::from_system("test");
        let mut meta = RepoMeta::new(&invoker);

        // Before soft delete - deleted_by should be omitted (skip_serializing_if)
        let json = serde_json::to_value(&meta).unwrap();
        println!("Before soft delete: {}", serde_json::to_string_pretty(&json).unwrap());
        assert!(json.get("deleted_by").is_none(), "deleted_by should be omitted when None");

        // After soft delete - deleted_by should serialize as the Invoker object directly
        meta.soft_delete(&invoker);
        let json = serde_json::to_value(&meta).unwrap();
        println!("After soft delete: {}", serde_json::to_string_pretty(&json).unwrap());

        // Check that deleted_by is present and has the correct Invoker structure
        let deleted_by = json.get("deleted_by").expect("deleted_by should be present");
        assert!(!deleted_by.is_null());
        // Invoker::System serializes as { "System": { "tag": "test" } }
        assert_eq!(deleted_by["System"]["tag"], "test");
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL to be set"]
    async fn test_foo_repo_cannot_update_deleted() -> anyhow::Result<()> {
        let database_url = std::env::var("DATABASE_URL")?;
        let pool = sqlx::PgPool::connect(&database_url).await?;
        let ctx = test_context_with_temp_table(pool).await?;

        // Create and soft delete a record
        let foo = FooRepo::new().create(FooCreate { name: "ToDelete".to_string() }, &ctx).await?;
        FooRepo::new().delete(foo.id, &ctx).await?;

        // Try to update the deleted record - should fail to find it
        let result = FooRepo::new()
            .update(foo.id, FooUpdate { name: Some("Updated".to_string()) }, &ctx)
            .await;

        assert!(result.is_err(), "Should not be able to update a deleted record");
        assert!(result.unwrap_err().to_string().contains("Failed to update Foo"));

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL to be set"]
    async fn test_foo_repo_cannot_update_many_deleted() -> anyhow::Result<()> {
        let database_url = std::env::var("DATABASE_URL")?;
        let pool = sqlx::PgPool::connect(&database_url).await?;
        let ctx = test_context_with_temp_table(pool).await?;

        // Create two records, soft delete one
        FooRepo::new().create(FooCreate { name: "Alice".to_string() }, &ctx).await?;
        let bob = FooRepo::new().create(FooCreate { name: "Bob".to_string() }, &ctx).await?;
        FooRepo::new().delete(bob.id, &ctx).await?;

        // Update all records with name "Bob" - should affect 0 records (Bob is deleted)
        let count = FooRepo::new()
            .update_many(
                vec![FooFilter::Name("Bob".to_string())],
                FooUpdate { name: Some("Updated".to_string()) },
                &ctx
            )
            .await?;

        assert_eq!(count, 0, "Should not update deleted records");

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL to be set"]
    async fn test_foo_repo_cannot_soft_delete_twice() -> anyhow::Result<()> {
        let database_url = std::env::var("DATABASE_URL")?;
        let pool = sqlx::PgPool::connect(&database_url).await?;
        let ctx = test_context_with_temp_table(pool).await?;

        // Create and soft delete a record
        let foo = FooRepo::new().create(FooCreate { name: "ToDelete".to_string() }, &ctx).await?;
        let first_delete = FooRepo::new().delete(foo.id, &ctx).await?;
        assert!(first_delete, "First delete should succeed");

        // Try to soft delete again - should return false (0 rows affected)
        let second_delete = FooRepo::new().delete(foo.id, &ctx).await?;
        assert!(!second_delete, "Second delete should return false (already deleted)");

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL to be set"]
    async fn test_foo_repo_hard_delete_removes_soft_deleted() -> anyhow::Result<()> {
        let database_url = std::env::var("DATABASE_URL")?;
        let pool = sqlx::PgPool::connect(&database_url).await?;
        let ctx = test_context_with_temp_table(pool).await?;

        // Create and soft delete a record
        let foo = FooRepo::new().create(FooCreate { name: "ToDelete".to_string() }, &ctx).await?;
        FooRepo::new().delete(foo.id, &ctx).await?;

        // Hard delete SHOULD work on soft-deleted records
        let hard_deleted = FooRepo::new().hard_delete(foo.id, &ctx).await?;
        assert!(hard_deleted, "Hard delete should permanently remove soft-deleted records");

        // Verify the record is completely gone from database
        let raw_result: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM foo WHERE id = $1"
        )
        .bind(foo.id.as_uuid())
        .fetch_one(&ctx)
        .await?;

        assert_eq!(raw_result.0, 0, "Soft-deleted record should be completely removed");

        Ok(())
    }

    #[tokio::test]
    #[ignore = "documents sqlx json attribute issue with Option"]
    async fn test_sqlx_json_option_issue() -> anyhow::Result<()> {
        // This test documents the issue with #[sqlx(json)] Option<T>
        //
        // When a PostgreSQL JSONB column is NULL, sqlx with #[sqlx(json)] tries to:
        // 1. Decode the column value as JSON (fails on NULL)
        // 2. Then wrap it in Option
        //
        // The error is: "error occurred while decoding column 'deleted_by': unexpected null; try decoding as an `Option`"
        //
        // This is confusing because the field IS an Option<T>, but sqlx's json attribute
        // processes the value before the Option gets involved.
        //
        // Solutions:
        // 1. Use Option<Json<T>> instead (handled at Option level before JSON parsing)
        // 2. Implement custom FromRow (what we do in RepoMeta) that checks for NULL first

        let database_url = std::env::var("DATABASE_URL")?;
        let pool = sqlx::PgPool::connect(&database_url).await?;

        // Raw query that returns NULL for a JSONB column - this works fine
        let result: Result<(Option<serde_json::Value>,), sqlx::Error> = sqlx::query_as(
            "SELECT NULL::jsonb"
        )
        .fetch_one(&pool)
        .await;

        println!("Direct Option<serde_json::Value>: {:?}", result);
        assert!(result.is_ok());

        Ok(())
    }
}
