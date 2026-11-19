-- The athlete/user id (strava_users.id, and the webhook queue's owner copy)
-- is 64-bit. strava_events.object_id is already bigint (the activity id).

ALTER TABLE strava_users
    ALTER COLUMN id TYPE bigint;

ALTER TABLE strava_events
    ALTER COLUMN owner_id TYPE bigint;
