-- Add migration script here

-- Track user activities (OpenShock control actions, etc.)
create table "activity" (
    id uuid primary key,
    occurred_at timestamptz not null default now(),
    user_id uuid not null references "user"(id) on delete cascade,
    
    -- Store the invoker context (User or System) as JSONB
    created_by jsonb not null,
    
    -- Store the activity details as JSONB
    action jsonb not null
);

-- Index for efficient querying by time ranges
create index idx_activity_occurred_at on "activity" (occurred_at);

-- Index for efficient user-specific queries
create index idx_activity_user_id on "activity" (user_id);

-- GIN index on action JSONB for complex queries
create index idx_activity_action_gin on "activity" using gin (action);

-- Composite index for user activity queries within time ranges
create index idx_activity_user_time on "activity" (user_id, occurred_at);
