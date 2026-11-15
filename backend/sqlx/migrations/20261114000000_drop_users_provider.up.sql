-- Drop the users.provider column: the activity provider (strava | icu),
-- derived from the provider-internal user data — a row in the strava_users
-- table means Strava, a row in the icu_users table means icu (a user has
-- exactly one).
ALTER TABLE users DROP COLUMN provider;
