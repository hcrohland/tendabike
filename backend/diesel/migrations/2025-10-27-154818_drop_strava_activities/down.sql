CREATE TABLE strava_activities(
    id bigint PRIMARY KEY,
    tendabike_id integer NOT NULL,
    user_id integer NOT NULL
);

CREATE INDEX strava_activities_tendabike_id_idx ON strava_activities(tendabike_id int4_ops);

