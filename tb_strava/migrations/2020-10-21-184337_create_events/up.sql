-- Your SQL goes here
CREATE TABLE events (
    id SERIAL PRIMARY KEY,
    object_type text NOT NULL,
    object_id bigint NOT NULL,
    aspect_type text NOT NULL,
    updates text,
    owner_id integer NOT NULL,
    subscription_id integer NOT NULL,
    event_time bigint NOT NULL
);