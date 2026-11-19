ALTER TABLE strava_users
    ALTER COLUMN id TYPE integer;

ALTER TABLE strava_events
    ALTER COLUMN owner_id TYPE integer;
