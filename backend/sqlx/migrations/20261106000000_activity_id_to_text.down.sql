-- Reverse: strip the "strava:" prefix and cast back to bigint. Only valid
-- while every row is a strava activity.
ALTER TABLE activities
    ALTER COLUMN id TYPE bigint
    USING (regexp_replace(id, '^strava:', '')::bigint);
