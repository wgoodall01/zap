use crate::{
    context::Context,
    openshock::{Duration, Intensity},
};
use anyhow::{Context as _, Result};
use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::BTreeMap;
use strum::EnumDiscriminants;
use uuid::Uuid;

/// Represents different types of activities that can be logged in the system.
///
/// Each variant corresponds to an OpenShock control action that can be performed
/// on devices, with the relevant parameters for that action type.
#[derive(Debug, Clone, Serialize, Deserialize, EnumDiscriminants)]
#[strum_discriminants(name(ActivityType))]
#[strum_discriminants(derive(PartialOrd, Ord, Serialize, Deserialize))]
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

/// Activity counts.
#[derive(Debug, Clone, FromRow, Serialize, JsonSchema)]
pub struct UserActivityCount {
    pub user_id: Uuid,
    #[sqlx(json)]
    pub counts: BTreeMap<ActivityType, u64>,
    #[sqlx(try_from = "i64")]
    pub total_actions: u64,
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
            crate::context::Invoker::User { id } => id,
            crate::context::Invoker::System { .. } => {
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
            insert into activity (id, user_id, created_by, action)
            values ($1, $2, $3, $4)
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
        reachback: chrono::TimeDelta,
        top_n: u32,
    ) -> Result<Vec<UserActivityCount>> {
        let cutoff_time = Utc::now() - reachback;

        // Query all activities within the time window
        let results: Vec<UserActivityCount> = sqlx::query_as(
            r#"
            with

            -- Get all activity over the time period described.
            log as (
                select
                    (created_by->'User'->>'id')::uuid as user_id,
                    k as action,
                    count(*) as count
                from activity, jsonb_object_keys(action) as k
                where true
                    and occurred_at > $1
                group by 1, 2
            )

            -- Aggregate to `{ [activity]: count }` per user, map to struct with column names.
            select
                user_id as "user_id",
                jsonb_object_agg(action, count::int) as "counts",
                sum(count)::bigint as "total_actions"
            from log
            group by user_id
            order by "total_actions" desc
            limit $2
            ;
            "#,
        )
        .bind(cutoff_time)
        .bind(top_n as i64)
        .fetch_all(ctx)
        .await
        .context("Failed to fetch activity records")?;

        Ok(results)
    }
}
