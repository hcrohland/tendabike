
CREATE TABLE activity_types (
    id SERIAL PRIMARY KEY,
    name text NOT NULL,
    gear integer NOT NULL REFERENCES part_types(id)
);

CREATE TABLE activities (
    id SERIAL PRIMARY KEY,
    user_id integer NOT NULL,
    what integer REFERENCES activity_types(id),
    name text NOT NULL,
    start timestamp with time zone NOT NULL DEFAULT now(),
    duration integer NOT NULL,
    time integer,
    distance integer,
    climb integer,
    descend integer,
    power integer,
    gear integer,
    registered BOOLEAN NOT NULL DEFAULT FALSE
);
