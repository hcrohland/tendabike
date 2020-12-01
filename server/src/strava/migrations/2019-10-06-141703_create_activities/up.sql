-- Your SQL goes here
CREATE TABLE activities (
    id bigint PRIMARY KEY,
    tendabike_id integer NOT NULL,
    user_id integer NOT NULL REFERENCES users(tendabike_id)
);

CREATE INDEX activities_tendabike_id_idx ON activities(tendabike_id int4_ops);
