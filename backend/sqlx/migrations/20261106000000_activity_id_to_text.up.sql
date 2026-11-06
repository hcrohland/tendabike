-- activities.id is a provider-prefixed text id. Existing rows are all
-- Strava activities, so they are backfilled with the "strava:" prefix.
ALTER TABLE activities
    ALTER COLUMN id TYPE text
    USING 'strava:' || id::text;
