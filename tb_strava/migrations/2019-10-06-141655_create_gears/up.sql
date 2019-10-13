-- Your SQL goes here
CREATE TABLE gears (
    id text PRIMARY KEY,
    tendabike_id integer,
    user_id integer NOT NULL REFERENCES users(id)
);
