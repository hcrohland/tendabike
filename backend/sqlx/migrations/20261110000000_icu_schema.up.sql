-- intervals.icu schema (spec #763): the second activity provider.
-- Adds the user's provider column and the icu_users / icu_events tables.

-- ============================================================================
-- USERS.PROVIDER COLUMN
-- ============================================================================
-- One provider per user, set at signup, immutable. Values: 'strava' | 'icu'.
ALTER TABLE users
    ADD COLUMN provider text NOT NULL DEFAULT 'strava';

COMMENT ON COLUMN users.provider IS 'The activity provider (strava | icu); set at signup, immutable';

-- ============================================================================
-- ICU_USERS TABLE
-- ============================================================================
-- Mirrors strava_users. The athlete id is a provider text id (intervals.icu
-- ids are not integers). No refresh token: intervals.icu tokens are durable.
CREATE TABLE IF NOT EXISTS icu_users(
    id serial PRIMARY KEY,
    tbid integer NOT NULL UNIQUE,
    icu_athlete_id text NOT NULL UNIQUE,
    access_token text,
    disabled boolean NOT NULL DEFAULT false
);

COMMENT ON TABLE icu_users IS 'intervals.icu integration user mappings';

-- ============================================================================
-- ICU_EVENTS TABLE
-- ============================================================================
-- Mirrors strava_events (the webhook queue). The event_time doubles as the
-- backfill cursor.
CREATE TABLE IF NOT EXISTS icu_events(
    id serial PRIMARY KEY,
    icu_athlete_id text NOT NULL,
    activity_id text NOT NULL,
    event_time bigint NOT NULL,
    object_type text NOT NULL
);

-- Create indexes for common query patterns
CREATE INDEX IF NOT EXISTS icu_events_athlete_id_idx ON icu_events(icu_athlete_id);
CREATE INDEX IF NOT EXISTS icu_events_time_idx ON icu_events(event_time);

COMMENT ON TABLE icu_events IS 'intervals.icu webhook events queue';
