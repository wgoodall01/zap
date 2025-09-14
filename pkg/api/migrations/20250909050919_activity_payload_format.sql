-- Add migration script here

-- Changes we made:
-- 1. We changed `serde` from adjacently-tagged to internally-tagged enum.
--    This means we need to drop all existing activities, as they will fail to deserialize.
-- 2. We changed the format of `created_by`. Truncate everything.
truncate table activity cascade;
truncate table login_tg cascade;
truncate table "user" cascade;
