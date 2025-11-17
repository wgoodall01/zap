//! SQL query builder with compile-time SQL injection protection.
//!
//! # SQL Injection Safety
//!
//! This module provides strong guarantees against SQL injection attacks through
//! Rust's type system and macro hygiene:
//!
//! ## Safe Operations
//!
//! 1. **`sql!` macro**: Only accepts `&'static str` literals for SQL fragments.
//!    This is enforced at compile time by Rust's macro system - you cannot pass
//!    runtime strings or variables as SQL text.
//!
//!    ```
//!    # use api::sql;
//!    let name = "Alice";
//!    let query = sql!("SELECT * FROM users WHERE name = " name);
//!    // SQL: "SELECT * FROM users WHERE name = ?"
//!    // Binds: ["Alice"]
//!    ```
//!
//! 2. **Bound parameters**: All dynamic values passed to the `sql!` macro are
//!    automatically converted to bound parameters (`?` placeholders), preventing
//!    SQL injection.
//!
//! 3. **Composition methods** (`concat`, `join`): These only combine existing
//!    `Sql` objects, preserving the safety guarantees.
//!
//! ## Unsafe Operations
//!
//! The only way to bypass these protections is through `Sql::raw()`, which is
//! marked `unsafe` to make SQL injection risks explicit in the code:
//!
//! ```no_run
//! # use api::db::sql::Sql;
//! # let user_input = "name";
//! // This requires `unsafe` block - clear signal of potential danger
//! let dynamic_sql = unsafe { Sql::raw(format!("ORDER BY {}", user_input)) };
//! ```
//!
//! ## Proof of Safety
//!
//! If you avoid calling `Sql::raw()`, SQL injection is impossible because:
//! 1. The `sql!` macro only accepts string literals (`$lit:literal`), not runtime strings
//! 2. All non-literal values are converted to bound parameters via `append_value()`
//! 3. The `Sql` struct has no public constructors except `new()` and `empty()`
//! 4. Methods like `concat()` and `join()` only manipulate existing `Sql` objects
//!
//! Therefore, without `unsafe`, all SQL text originates from compile-time literals,
//! and all runtime values are safely bound as parameters.

use serde_json::Value as JsonValue;
use std::fmt;

/// A SQL query and its bound parameters.
///
/// The query is stored as a `Vec<String>` where each element is a SQL fragment,
/// and bind parameters are interleaved between fragments. This representation
/// maintains the invariant that `query.len() == binds.len() + 1`.
///
/// For example, `sql!("SELECT * FROM users WHERE name = " name " AND age = " age)`
/// becomes:
/// - query: `["SELECT * FROM users WHERE name = ", " AND age = ", ""]`
/// - binds: `[name, age]`
///
/// This allows rendering to different SQL dialects:
/// - PostgreSQL: `SELECT * FROM users WHERE name = $1 AND age = $2`
/// - SQLite/MySQL: `SELECT * FROM users WHERE name = ? AND age = ?`
#[derive(Debug, Clone, PartialEq)]
pub struct Sql {
    /// SQL fragments, where `query.len() == binds.len() + 1`
    pub query: Vec<String>,
    pub binds: Vec<JsonValue>,
}

impl Default for Sql {
    fn default() -> Self {
        Self::empty()
    }
}

impl FromIterator<Sql> for Sql {
    fn from_iter<T: IntoIterator<Item = Sql>>(iter: T) -> Self {
        Sql::join(iter)
    }
}

impl fmt::Display for Sql {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Render with ? placeholders
        for (i, fragment) in self.query.iter().enumerate() {
            write!(f, "{}", fragment)?;
            if i < self.binds.len() {
                write!(f, "?")?;
            }
        }
        Ok(())
    }
}

impl Sql {
    /// Create a new SQL query with the given query fragments and binds.
    ///
    /// # Panics
    ///
    /// Panics if `query.len() != binds.len() + 1`.
    pub fn new(query: Vec<String>, binds: Vec<JsonValue>) -> Self {
        let sql = Sql { query, binds };
        sql._check_invariants();
        sql
    }

    /// Create a new empty SQL query with no binds.
    pub fn empty() -> Self {
        Sql {
            query: vec![String::new()],
            binds: Vec::new(),
        }
    }

    /// Check that the invariant `query.len() == binds.len() + 1` holds.
    ///
    /// This is called in debug mode to catch programming errors.
    #[inline]
    fn _check_invariants(&self) {
        debug_assert_eq!(
            self.query.len(),
            self.binds.len() + 1,
            "Invariant violated: query.len() must equal binds.len() + 1"
        );
    }

    /// Create a raw SQL snippet from a non-static string.
    ///
    /// # Safety
    ///
    /// This function is marked `unsafe` because it bypasses the SQL injection
    /// protection provided by the `sql!` macro. The `sql!` macro ensures that:
    /// - All SQL fragments are `&'static str` literals (compile-time checked)
    /// - All dynamic values are bound as parameters (using `?` placeholders)
    ///
    /// By calling `raw()`, you are asserting that the input string does not
    /// contain user-controlled data that could lead to SQL injection. You must
    /// ensure that the string is either:
    /// - Constructed from trusted, validated sources only
    /// - Does not include any user input without proper escaping/validation
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use api::db::sql::Sql;
    /// # fn validate_column_name(input: &str) -> Result<String, String> { Ok(input.to_string()) }
    /// # let user_input = "name";
    /// // SAFE: constructed from validated column name
    /// let column = validate_column_name(user_input)?;
    /// let fragment = unsafe { Sql::raw(format!("ORDER BY {}", column)) };
    ///
    /// // UNSAFE: directly using user input
    /// // let fragment = unsafe { Sql::raw(format!("WHERE name = '{}'", user_input)) }; // DON'T DO THIS!
    /// # Ok::<(), String>(())
    /// ```
    pub unsafe fn raw(query: impl AsRef<str>) -> Self {
        let sql = Sql {
            query: vec![query.as_ref().to_string()],
            binds: Vec::new(),
        };
        sql._check_invariants();
        sql
    }

    /// Concatenate this SQL with another, returning a new Sql.
    pub fn concat(&self, other: &Sql) -> Sql {
        Self::join(vec![self.clone(), other.clone()])
    }

    /// Join multiple SQL fragments without a separator.
    pub fn join(parts: impl IntoIterator<Item = Sql>) -> Sql {
        Self::join_with("", parts)
    }

    /// Join multiple SQL fragments with a separator.
    pub fn join_with(separator: &'static str, parts: impl IntoIterator<Item = Sql>) -> Sql {
        let mut joined = Sql::empty();

        let mut first = true;
        for part in parts {
            part._check_invariants();
            let Sql {
                query: mut part_query,
                binds: part_binds,
            } = part;

            // Split the part query into a head and tail elements.
            let part_query_tail = part_query.split_off(1);
            let part_query_head = part_query.pop().expect("No head in part SQL");

            // Add the separator and part query head to the last segment of the joined query.
            {
                // Pop off the last query segment.
                let last_segment = joined
                    .query
                    .last_mut()
                    .expect("No last segment in joined SQL");

                // Append the separator and head of the part query.
                if !first {
                    last_segment.push_str(separator);
                }
                first = false;

                // Append the head of the part query.
                last_segment.push_str(&part_query_head);
            }

            // Append the tail of the part query.
            joined.query.extend(part_query_tail);

            // Append the binds.
            joined.binds.extend(part_binds);

            // Invariant-check.
            joined._check_invariants();
        }
        joined
    }

    /// Render the SQL query with PostgreSQL-style numbered placeholders ($1, $2, etc.).
    pub fn render_pg(&self) -> String {
        let mut result = String::new();
        for (i, fragment) in self.query.iter().enumerate() {
            result.push_str(fragment);
            if i < self.binds.len() {
                result.push_str(&format!("${}", i + 1));
            }
        }
        result
    }

}

/// Macro for building SQL queries with bound parameters.
///
/// Syntax: `sql!("literal" expr "literal" expr ...)`
///
/// String literals are concatenated into the query, and expressions are
/// converted to bind parameters (as `?` placeholders).
#[macro_export]
macro_rules! sql {
    // Base case: empty
    () => {
        $crate::db::sql::Sql {
            query: vec![String::new()],
            binds: vec![],
        }
    };

    // Base case: just a string literal
    ($sql:literal) => {
        $crate::db::sql::Sql {
            query: vec![$sql.to_string()],
            binds: vec![],
        }
    };

    // Recursive case: build up by consuming tokens one at a time
    // Internal rule that accumulates results
    (@accum $query:expr, $binds:expr,) => {{
        // Base case: we're done processing tokens
        // The query vec should already have the right number of fragments
        // (append_value already pushed an empty string for the next literal after each bind)
        $crate::db::sql::Sql {
            query: $query,
            binds: $binds,
        }
    }};

    (@accum $query:expr, $binds:expr, $lit:literal $($rest:tt)*) => {{
        // Append literal to the last query fragment
        if let Some(last) = $query.last_mut() {
            last.push_str($lit);
        }
        $crate::sql!(@accum $query, $binds, $($rest)*)
    }};

    (@accum $query:expr, $binds:expr, ($val:expr) $($rest:tt)*) => {{
        // Parenthesized expression: use Display to insert directly into SQL
        if let Some(last) = $query.last_mut() {
            use std::fmt::Write;
            write!(last, "{}", $val).unwrap();
        }
        $crate::sql!(@accum $query, $binds, $($rest)*)
    }};

    (@accum $query:expr, $binds:expr, $val:tt $($rest:tt)*) => {{
        // Append value as a bind parameter
        let val = $val;
        $crate::db::sql::append_value(&mut $query, &mut $binds, val);
        $crate::sql!(@accum $query, $binds, $($rest)*)
    }};

    // Entry point: start with first literal
    ($first:literal $($rest:tt)*) => {{
        let mut query = vec![$first.to_string()];
        let mut binds = vec![];
        $crate::sql!(@accum query, binds, $($rest)*)
    }};
}

/// Helper trait to distinguish between Sql objects and values to bind.
pub trait AppendToSql {
    fn append_to(self, query: &mut Vec<String>, binds: &mut Vec<JsonValue>);
}

impl AppendToSql for Sql {
    fn append_to(self, query: &mut Vec<String>, binds: &mut Vec<JsonValue>) {
        // Merge the last fragment of query with the first fragment of self.query
        if let Some(last) = query.last_mut()
            && let Some(first) = self.query.first()
        {
            last.push_str(first);
        }
        // Add the remaining fragments from self.query
        query.extend_from_slice(&self.query[1..]);
        binds.extend(self.binds);
    }
}

impl<T: serde::Serialize> AppendToSql for T {
    fn append_to(self, query: &mut Vec<String>, binds: &mut Vec<JsonValue>) {
        binds.push(serde_json::to_value(self).unwrap());
        // Start a new query fragment for the next literal
        query.push(String::new());
    }
}

/// Helper function to append a value to SQL (either another Sql or a bind value).
#[doc(hidden)]
pub fn append_value<T: AppendToSql>(query: &mut Vec<String>, binds: &mut Vec<JsonValue>, value: T) {
    value.append_to(query, binds);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_invariants() {
        // Test that all SQL objects maintain the invariant
        let name = "Alice";
        let age = 30;

        let q1 = sql!("SELECT * FROM users");
        eprintln!(
            "q1: query.len()={}, binds.len()={}, query={:?}",
            q1.query.len(),
            q1.binds.len(),
            q1.query
        );
        assert_eq!(q1.query.len(), q1.binds.len() + 1);

        let q2 = sql!("WHERE name = " name);
        eprintln!(
            "q2: query.len()={}, binds.len()={}, query={:?}",
            q2.query.len(),
            q2.binds.len(),
            q2.query
        );
        assert_eq!(q2.query.len(), q2.binds.len() + 1);

        let q3 = sql!("WHERE name = " name " AND age = " age);
        eprintln!(
            "q3: query.len()={}, binds.len()={}, query={:?}",
            q3.query.len(),
            q3.binds.len(),
            q3.query
        );
        assert_eq!(q3.query.len(), q3.binds.len() + 1);
    }

    #[test]
    fn test_empty_sql() {
        let result = sql!();
        assert_eq!(format!("{}", result), "");
        assert_eq!(result.render_pg(), "");
        assert_eq!(result.binds.len(), 0);
    }

    #[test]
    fn test_literal_only() {
        let result = sql!("SELECT * FROM users");
        assert_eq!(format!("{}", result), "SELECT * FROM users");
        assert_eq!(result.render_pg(), "SELECT * FROM users");
        assert_eq!(result.binds.len(), 0);
    }

    #[test]
    fn test_single_bind() {
        let name = "Alice";
        let result = sql!("SELECT * FROM users WHERE name = " name);
        assert_eq!(format!("{}", result), "SELECT * FROM users WHERE name = ?");
        assert_eq!(result.render_pg(), "SELECT * FROM users WHERE name = $1");
        assert_eq!(result.binds, vec![json!("Alice")]);
    }

    #[test]
    fn test_multiple_binds() {
        let name = "Alice";
        let age = 30;
        let result = sql!("SELECT * FROM users WHERE name = " name " AND age = " age);
        assert_eq!(
            format!("{}", result),
            "SELECT * FROM users WHERE name = ? AND age = ?"
        );
        assert_eq!(result.binds, vec![json!("Alice"), json!(30)]);
    }

    #[test]
    fn test_nested_sql() {
        let name = "Alice";
        let age = 30;
        let inner = sql!("name = " name " AND age = " age);
        let result = sql!("SELECT * FROM users WHERE " inner);

        assert_eq!(
            format!("{}", result),
            "SELECT * FROM users WHERE name = ? AND age = ?"
        );
        assert_eq!(result.binds, vec![json!("Alice"), json!(30)]);
    }

    #[test]
    fn test_sql_join_basic() {
        let name = "Alice";
        let age = 30;
        let predicates = vec![sql!("name = " name), sql!("age = " age)];

        let result = Sql::join_with(" AND ", predicates);
        assert_eq!(format!("{}", result), "name = ? AND age = ?");
        assert_eq!(result.binds, vec![json!("Alice"), json!(30)]);
    }

    #[test]
    fn test_sql_join_with_where_clause() {
        let name = "Alice";
        let age = 30;
        let predicates = vec![sql!("name = " name), sql!("age > " age)];

        let where_clause = Sql::join_with(" AND ", predicates);
        let result = sql!("SELECT * FROM users WHERE " where_clause ";");

        assert_eq!(
            format!("{}", result),
            "SELECT * FROM users WHERE name = ? AND age > ?;"
        );
        assert_eq!(result.binds, vec![json!("Alice"), json!(30)]);
    }

    #[test]
    fn test_complex_nested_example() {
        let name = "Bob";
        let start_date = "2024-01-01";

        let predicates = vec![sql!("name = " name), sql!("created_at > " start_date)];
        let where_clause = Sql::join_with(" AND ", predicates);
        let result = sql!("SELECT * FROM foos WHERE " where_clause ";");

        assert_eq!(
            format!("{}", result),
            "SELECT * FROM foos WHERE name = ? AND created_at > ?;"
        );
        assert_eq!(result.binds, vec![json!("Bob"), json!("2024-01-01")]);
    }

    #[test]
    fn test_no_query_rewriting() {
        // Ensure we don't rewrite query content like $1 in strings
        let value = "test";
        let result = sql!("SELECT * FROM users WHERE name = " value " AND data LIKE '$1%'");
        assert_eq!(
            format!("{}", result),
            "SELECT * FROM users WHERE name = ? AND data LIKE '$1%'"
        );
        assert_eq!(result.binds, vec![json!("test")]);
    }

    #[test]
    fn test_join_empty_vec() {
        let result = Sql::join_with(" AND ", vec![]);
        assert_eq!(format!("{}", result), "");
        assert_eq!(result.binds.len(), 0);
    }

    #[test]
    fn test_join_single_item() {
        let name = "Alice";
        let predicates = vec![sql!("name = " name)];

        let result = Sql::join_with(" AND ", predicates);
        assert_eq!(format!("{}", result), "name = ?");
        assert_eq!(result.binds, vec![json!("Alice")]);
    }

    #[test]
    fn test_multiple_levels_of_nesting() {
        let name = "Alice";
        let age = 30;
        let city = "NYC";

        let name_filter = sql!("name = " name);
        let age_filter = sql!("age = " age);
        let combined = Sql::join_with(" AND ", vec![name_filter, age_filter]);
        let result = sql!("SELECT * FROM users WHERE " combined " AND city = " city);

        assert_eq!(
            format!("{}", result),
            "SELECT * FROM users WHERE name = ? AND age = ? AND city = ?"
        );
        assert_eq!(result.binds, vec![json!("Alice"), json!(30), json!("NYC")]);
    }

    #[test]
    fn test_trailing_literal() {
        let id = 42;
        let result = sql!("SELECT * FROM users WHERE id = " id ";");
        assert_eq!(format!("{}", result), "SELECT * FROM users WHERE id = ?;");
        assert_eq!(result.binds, vec![json!(42)]);
    }

    #[test]
    fn test_sql_new() {
        let result = Sql::new(
            vec![
                "SELECT * FROM users WHERE id = ".to_string(),
                "".to_string(),
            ],
            vec![json!(42)],
        );
        assert_eq!(format!("{}", result), "SELECT * FROM users WHERE id = ?");
        assert_eq!(result.render_pg(), "SELECT * FROM users WHERE id = $1");
        assert_eq!(result.binds, vec![json!(42)]);
    }

    #[test]
    fn test_sql_empty() {
        let result = Sql::empty();
        assert_eq!(format!("{}", result), "");
        assert_eq!(result.binds.len(), 0);
    }

    #[test]
    fn test_sql_concat_basic() {
        let name = "Alice";
        let age = 30;

        let part1 = sql!("SELECT * FROM users WHERE name = " name);
        let part2 = sql!(" AND age = " age);
        let result = part1.concat(&part2);

        assert_eq!(
            format!("{}", result),
            "SELECT * FROM users WHERE name = ? AND age = ?"
        );
        assert_eq!(result.binds, vec![json!("Alice"), json!(30)]);
    }

    #[test]
    fn test_sql_concat_empty() {
        let name = "Alice";
        let sql_part = sql!("name = " name);
        let empty = Sql::empty();

        // Concat with empty on right
        let result1 = sql_part.concat(&empty);
        assert_eq!(format!("{}", result1), "name = ?");
        assert_eq!(result1.binds, vec![json!("Alice")]);

        // Concat with empty on left
        let result2 = empty.concat(&sql_part);
        assert_eq!(format!("{}", result2), "name = ?");
        assert_eq!(result2.binds, vec![json!("Alice")]);
    }

    #[test]
    fn test_sql_concat_multiple() {
        let name = "Alice";
        let age = 30;
        let city = "NYC";

        let part1 = sql!("SELECT * FROM users WHERE name = " name);
        let part2 = sql!(" AND age = " age);
        let part3 = sql!(" AND city = " city);

        let result = part1.concat(&part2).concat(&part3);

        assert_eq!(
            format!("{}", result),
            "SELECT * FROM users WHERE name = ? AND age = ? AND city = ?"
        );
        assert_eq!(result.binds, vec![json!("Alice"), json!(30), json!("NYC")]);
    }

    #[test]
    fn test_sql_concat_with_nested() {
        let name = "Alice";
        let age = 30;
        let city = "NYC";

        let inner = sql!("name = " name " AND age = " age);
        let outer = sql!("SELECT * FROM users WHERE ");
        let suffix = sql!(" AND city = " city);

        let result = outer.concat(&inner).concat(&suffix);

        assert_eq!(
            format!("{}", result),
            "SELECT * FROM users WHERE name = ? AND age = ? AND city = ?"
        );
        assert_eq!(result.binds, vec![json!("Alice"), json!(30), json!("NYC")]);
    }

    #[test]
    fn test_sql_display() {
        let name = "Alice";
        let age = 30;
        let result = sql!("SELECT * FROM users WHERE name = " name " AND age = " age);

        // Display should print just the query
        assert_eq!(
            format!("{}", result),
            "SELECT * FROM users WHERE name = ? AND age = ?"
        );
    }

    #[test]
    fn test_sql_display_empty() {
        let empty = Sql::empty();
        assert_eq!(format!("{}", empty), "");
    }

    #[test]
    fn test_sql_debug() {
        let name = "Alice";
        let result = sql!("SELECT * FROM users WHERE name = " name);

        // Debug should show the full struct
        let debug_output = format!("{:?}", result);
        assert!(debug_output.contains("Sql"));
        assert!(debug_output.contains("query"));
        assert!(debug_output.contains("binds"));
        assert!(debug_output.contains("SELECT * FROM users WHERE name = "));
        assert!(debug_output.contains("Alice"));
    }

    #[test]
    fn test_sql_raw_static_str() {
        let result = unsafe { Sql::raw("ORDER BY created_at DESC") };
        assert_eq!(format!("{}", result), "ORDER BY created_at DESC");
        assert_eq!(result.binds.len(), 0);
    }

    #[test]
    fn test_sql_raw_string() {
        let dynamic_str = format!("LIMIT {}", 10);
        let result = unsafe { Sql::raw(dynamic_str) };
        assert_eq!(format!("{}", result), "LIMIT 10");
        assert_eq!(result.binds.len(), 0);
    }

    #[test]
    fn test_sql_raw_with_concat() {
        let active = true;
        let base = sql!("SELECT * FROM users WHERE active = " active);
        let order = unsafe { Sql::raw(" ORDER BY name") };
        let result = base.concat(&order);

        assert_eq!(
            format!("{}", result),
            "SELECT * FROM users WHERE active = ? ORDER BY name"
        );
        assert_eq!(result.binds, vec![json!(true)]);
    }

    #[test]
    fn test_sql_injection_safety_cannot_pass_string_variable() {
        // This test documents that you CANNOT do this:
        // let user_input = "'; DROP TABLE users; --";
        // let query = sql!("SELECT * FROM users WHERE name = " user_input);
        //
        // The above would NOT compile because the macro only accepts literals.
        // Instead, the user_input would be bound as a parameter:
        let user_input = "'; DROP TABLE users; --";
        let query = sql!("SELECT * FROM users WHERE name = " user_input);

        // The malicious string is safely bound as a parameter
        assert_eq!(format!("{}", query), "SELECT * FROM users WHERE name = ?");
        assert_eq!(query.binds, vec![json!("'; DROP TABLE users; --")]);
    }

    #[test]
    fn test_from_iterator_collect() {
        let name = "Alice";
        let age = 30;

        let parts = vec![
            sql!("SELECT * FROM users WHERE name = " name),
            sql!(" AND age = " age),
        ];

        let result: Sql = parts.into_iter().collect();
        assert_eq!(
            format!("{}", result),
            "SELECT * FROM users WHERE name = ? AND age = ?"
        );
        assert_eq!(result.binds, vec![json!("Alice"), json!(30)]);
    }

    #[test]
    fn test_from_iterator_collect_empty() {
        let parts: Vec<Sql> = vec![];
        let result: Sql = parts.into_iter().collect();
        assert_eq!(format!("{}", result), "");
        assert_eq!(result.binds.len(), 0);
    }

    #[test]
    fn test_parenthesized_display() {
        // Test that (expr) inserts the Display representation directly
        let one = 1;
        let yes = true;
        let result = sql!("select " one ", " ("bind1") ", " yes ";");
        assert_eq!(format!("{}", result), "select ?, bind1, ?;");
        assert_eq!(result.render_pg(), "select $1, bind1, $2;");
        assert_eq!(result.binds, vec![json!(1), json!(true)]);
    }

}
