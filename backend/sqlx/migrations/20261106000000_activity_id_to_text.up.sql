-- activities.id moves from bigint to a provider-prefixed text id.
-- Existing rows are all Strava activities, so they are backfilled with the
-- "strava:" prefix (issue #761).
ALTER TABLE activities
    ALTER COLUMN id TYPE text
    USING 'strava:' || id::text;
