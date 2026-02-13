use crate::db::id::Id;
use crate::db::sql::Sql;
use crate::sql;
use anyhow::{Result, anyhow, bail};

/// Trait for filters that can be applied to a result set.
pub trait Filter {
    /// The record type (e.g. `Foo`) that this filter applies to.
    /// This type is unused in the trait, but allows for better type checking.
    type Record;

    /// Convert the filter into a SQL boolean-valued fragment, which can be included in a WHERE clause.
    fn as_predicate(&self) -> Sql;
}

/// A table-valued SQL query, and its expected result row type.
#[derive(Clone, Debug)]
pub struct ResultSet<T> {
    /// Argument of the FROM clause. Commonly just the table name.
    from: Sql,

    /// Predicate for the WHERE clause, joined with AND.
    predicate: Vec<Sql>,

    /// Optional LIMIT clause.
    limit: Option<i64>,

    _phantom: std::marker::PhantomData<T>,
}

impl<T> ResultSet<T> {
    /// Create a new ResultSet from a FROM clause.
    pub fn new(from: Sql) -> Self {
        Self {
            from,
            predicate: Vec::new(),
            limit: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Add a filter to the ID column and a limit=1.
    pub fn by_id(mut self, id: impl Id) -> Self {
        let uuid = id.as_uuid();
        self.predicate.push(sql!("id = " uuid));
        self.limit = Some(1);
        self
    }

    /// Add a filter to this result set, which will be combined with AND in the WHERE clause.
    pub fn filter<F>(mut self, filter: F) -> Self
    where
        F: Filter<Record = T>,
    {
        self.predicate.push(filter.as_predicate());
        self
    }

    /// Set a LIMIT clause on this result set.
    pub fn limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Add a raw SQL predicate to the WHERE clause.
    /// This is useful for adding predicates that don't correspond to a Filter variant.
    pub fn where_sql(mut self, predicate: Sql) -> Self {
        self.predicate.push(predicate);
        self
    }
}

impl<T> ResultSet<T>
where
    T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    /// Execute the query and return all results.
    pub async fn fetch_all<'c, C>(self, conn: C) -> Result<Vec<T>, sqlx::Error>
    where
        C: sqlx::Executor<'c, Database = sqlx::Postgres>,
    {
        // Build the query
        let from = self.from;
        let mut query = sql!("SELECT * FROM " from);

        // Add WHERE clause if there are predicates
        if !self.predicate.is_empty() {
            let where_clause = Sql::join_with(" AND ", self.predicate);
            query = query.concat(&sql!(" WHERE " where_clause));
        }

        // Add LIMIT clause if specified
        if let Some(limit) = self.limit {
            query = query.concat(&sql!(" LIMIT " limit));
        }

        let query_str = query.render_pg();

        // Build the sqlx query with binds using the helper method
        let sqlx_query = sqlx::query_as::<_, T>(&query_str);
        let sqlx_query = query.bind_to_query_as(sqlx_query);

        sqlx_query.fetch_all(conn).await
    }

    /// Execute the query, assert that only one result matches, and return it.
    pub async fn try_fetch_one<'c, C>(mut self, conn: C) -> Result<Option<T>>
    where
        C: sqlx::Executor<'c, Database = sqlx::Postgres>,
    {
        self.limit = Some(2);
        let results = self.fetch_all(conn).await?;
        match results.len() {
            0 => Ok(None),
            1 => Ok(Some(results.into_iter().next().unwrap())),
            _ => bail!("Expected only one result, found two or more"),
        }
    }

    /// Execute the query and return the first matching result, if any.
    pub async fn fetch_one<'c, C>(self, conn: C) -> Result<T>
    where
        C: sqlx::Executor<'c, Database = sqlx::Postgres>,
    {
        self.try_fetch_one(conn)
            .await?
            .ok_or_else(|| anyhow!("No results found"))
    }

    /// Get the first matching result of the query.
    pub async fn try_fetch_first<'c, C>(mut self, conn: C) -> Result<Option<T>>
    where
        C: sqlx::Executor<'c, Database = sqlx::Postgres>,
    {
        self.limit = Some(1);
        let results = self.fetch_all(conn).await?;
        Ok(results.into_iter().next())
    }

    /// Get the first matching result of the query, or error if none found.
    pub async fn fetch_first<'c, C>(self, conn: C) -> Result<T>
    where
        C: sqlx::Executor<'c, Database = sqlx::Postgres>,
    {
        self.try_fetch_first(conn)
            .await?
            .ok_or_else(|| anyhow!("No results found"))
    }
}
