-- Drop the users.provider column (issue #870): the provider is no longer a
-- domain concept. It is derived from the provider-internal user data — a row
-- in the strava_users table means Strava, a row in the icu_users table means
-- icu (a user has exactly one).
ALTER TABLE users DROP COLUMN provider;
