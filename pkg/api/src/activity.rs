use crate::{
    context::Context,
    openshock::{Duration, Intensity},
};
use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Represents different types of activities that can be logged in the system.
///
/// Each variant corresponds to an OpenShock control action that can be performed
/// on devices, with the relevant parameters for that action type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "params")]
pub enum Activity {
    /// Electrical shock delivered to device
    Shock {
        /// Intensity level (0-100)
        intensity: Intensity,
        /// Duration of shock (minimum 300ms)
        duration: Duration,
    },
    /// Vibration activated on device
    Vibrate {
        /// Intensity level (0-100)
        intensity: Intensity,
        /// Duration of vibration (minimum 300ms)
        duration: Duration,
    },
    /// Sound/beep played on device
    Beep {
        /// Intensity level (0-100)
        intensity: Intensity,
        /// Duration of beep (minimum 300ms)
        duration: Duration,
    },
    /// Stop command sent to device (no parameters)
    Stop,
}

/// Represents a logged activity record from the database.
///
/// Contains all the metadata about when and by whom an activity was performed,
/// along with the activity details themselves.
#[derive(Debug, Clone, FromRow)]
pub struct ActivityRecord {
    /// Unique identifier for this activity record
    pub id: Uuid,
    /// When the activity occurred
    pub occurred_at: DateTime<Utc>,
    /// Which user the activity was performed for/by
    pub user_id: Uuid,
    /// Who/what initiated this activity (serialized Invoker)
    pub created_by: serde_json::Value,
    /// The activity that was performed (serialized Activity)
    pub action: serde_json::Value,
}

/// Result structure for user activity counts over a time period.
///
/// Provides counts for each type of activity that a user has performed,
/// allowing for analysis of usage patterns and activity levels.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserActivityCount {
    /// The user this count applies to
    pub user_id: Uuid,
    /// Number of shock activities
    pub shock_count: i64,
    /// Number of vibrate activities
    pub vibrate_count: i64,
    /// Number of beep activities
    pub beep_count: i64,
    /// Number of stop activities
    pub stop_count: i64,
    /// Total activity count across all types
    pub total_count: i64,
}

/// Service for logging and querying user activities.
///
/// Provides methods to record activities in the database and retrieve
/// aggregated statistics about user activity patterns over time.
#[derive(Debug, Clone)]
pub struct ActivityService;

impl ActivityService {
    /// Creates a new ActivityService instance.
    pub fn new() -> Self {
        Self
    }

    /// Logs an activity to the database.
    ///
    /// Records the activity with the current timestamp, associating it with
    /// the user from the context and storing who/what initiated the action.
    ///
    /// # Arguments
    /// * `ctx` - Application context containing user info and database access
    /// * `activity` - The activity to log
    ///
    /// # Returns
    /// The UUID of the created activity record.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The database operation fails
    /// - JSON serialization of the activity or invoker fails
    /// - The context doesn't contain a valid user
    pub async fn log(&self, ctx: &Context, activity: &Activity) -> Result<Uuid> {
        let activity_id = Uuid::now_v7();

        // Get user_id from context
        let user_id = match &ctx.invoker {
            crate::context::Invoker::User(user) => user.id,
            crate::context::Invoker::System(_) => {
                return Err(anyhow::anyhow!(
                    "Cannot log activity without a user context"
                ));
            }
        };

        // Serialize the invoker and activity to JSON
        let created_by =
            serde_json::to_value(&ctx.invoker).context("Failed to serialize invoker")?;
        let action = serde_json::to_value(activity).context("Failed to serialize activity")?;

        sqlx::query!(
            r#"
            INSERT INTO activity (id, user_id, created_by, action)
            VALUES ($1, $2, $3, $4)
            "#,
            activity_id,
            user_id,
            created_by,
            action
        )
        .execute(ctx)
        .await
        .context("Failed to insert activity record")?;

        Ok(activity_id)
    }

    /// Returns activity counts for all users within the specified time window.
    ///
    /// Aggregates activities by user and type, returning users sorted by
    /// their total activity count in descending order.
    ///
    /// # Arguments
    /// * `ctx` - Application context for database access
    /// * `reachback` - How far back in time to look for activities
    ///
    /// # Returns
    /// Vector of UserActivityCount structs, sorted by decreasing total activity.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The database query fails
    /// - JSON deserialization fails for any activity records
    pub async fn count_by_user(
        &self,
        ctx: &Context,
        reachback: chrono::Duration,
    ) -> Result<Vec<UserActivityCount>> {
        let cutoff_time = Utc::now() - reachback;

        // Query all activities within the time window
        let rows = sqlx::query_as!(
            ActivityRecord,
            r#"
            select id, occurred_at, user_id, created_by, action
            FROM activity
            WHERE occurred_at >= $1
            ORDER BY user_id, occurred_at
            "#,
            cutoff_time
        )
        .fetch_all(ctx)
        .await
        .context("Failed to fetch activity records")?;

        // Group by user and count activity types
        let mut user_counts: std::collections::HashMap<Uuid, UserActivityCount> =
            std::collections::HashMap::new();

        for row in rows {
            let activity: Activity = serde_json::from_value(row.action)
                .context("Failed to deserialize activity from database")?;

            let count = user_counts.entry(row.user_id).or_insert(UserActivityCount {
                user_id: row.user_id,
                shock_count: 0,
                vibrate_count: 0,
                beep_count: 0,
                stop_count: 0,
                total_count: 0,
            });

            match activity {
                Activity::Shock { .. } => count.shock_count += 1,
                Activity::Vibrate { .. } => count.vibrate_count += 1,
                Activity::Beep { .. } => count.beep_count += 1,
                Activity::Stop => count.stop_count += 1,
            }
            count.total_count += 1;
        }

        // Convert to vector and sort by total count (descending)
        let mut results: Vec<UserActivityCount> = user_counts.into_values().collect();
        results.sort_by(|a, b| b.total_count.cmp(&a.total_count));

        Ok(results)
    }
}
