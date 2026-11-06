-- Reverse: strip the "strava:" prefix and cast back to bigint. Only valid
-- while every row is a strava activity (the icu lane is not live yet,
-- issue #761).
ALTER TABLE activities
    ALTER COLUMN id TYPE bigint
    USING (regexp_replace(id, '^strava:', '')::bigint);
