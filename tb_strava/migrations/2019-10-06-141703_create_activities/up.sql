-- Your SQL goes here
CREATE TABLE activities (
    id bigint PRIMARY KEY,
    tendabike_id integer,
    user_id integer NOT NULL REFERENCES users(id),
    gear_id text REFERENCES gears(id)
);

CREATE INDEX activities_tendabike_id_idx ON activities(tendabike_id int4_ops);
