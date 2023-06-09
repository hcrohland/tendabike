CREATE TABLE strava_users (
    id integer PRIMARY KEY,
    tendabike_id integer NOT NULL UNIQUE,
    last_activity bigint Not Null,
    access_token text NOT NULL,
    expires_at bigint NOT NULL,
    refresh_token text NOT NULL
);
COMMENT ON COLUMN strava_users.id IS 'the id provided by Strava';

CREATE TABLE strava_gears (
    id text PRIMARY KEY,
    tendabike_id integer NOT NULL,
    user_id integer NOT NULL REFERENCES strava_users(tendabike_id)
);

CREATE TABLE strava_activities (
    id bigint PRIMARY KEY,
    tendabike_id integer NOT NULL,
    user_id integer NOT NULL REFERENCES strava_users(tendabike_id)
);

CREATE INDEX strava_activities_tendabike_id_idx ON strava_activities(tendabike_id int4_ops);

CREATE TABLE strava_events (
    id SERIAL PRIMARY KEY,
    object_type text NOT NULL,
    object_id bigint NOT NULL,
    aspect_type text NOT NULL,
    updates text,
    owner_id integer NOT NULL,
    subscription_id integer NOT NULL,
    event_time bigint NOT NULL
);