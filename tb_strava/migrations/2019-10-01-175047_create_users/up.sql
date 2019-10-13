CREATE TABLE users (
    id integer PRIMARY KEY,
    tendabike_id integer UNIQUE,
    last_activity bigint Not Null,
    access_token text NOT NULL,
    expires_at bigint NOT NULL,
    refresh_token text NOT NULL
);
COMMENT ON COLUMN users.id IS 'the id provided by Strava';



