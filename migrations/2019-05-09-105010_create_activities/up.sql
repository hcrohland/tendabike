
CREATE TABLE activity_types (
    id SERIAL PRIMARY KEY,
    name text NOT NULL,
    gear integer NOT NULL REFERENCES part_types(id)
);

CREATE TABLE activities (
    id SERIAL PRIMARY KEY,
    user_id integer NOT NULL,
    what integer NOT NULL REFERENCES activity_types(id),
    name text NOT NULL,
    start timestamp with time zone NOT NULL DEFAULT now(),
    duration integer NOT NULL,
    time integer,
    distance integer,
    climb integer,
    descend integer,
    power integer,
    gear integer
);

ALTER SEQUENCE activities_id_seq RESTART WITH 300;
ALTER SEQUENCE activitie_types_id_seq RESTART WITH 30;
