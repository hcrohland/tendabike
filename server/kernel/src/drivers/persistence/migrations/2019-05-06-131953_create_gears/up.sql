CREATE TABLE part_types (
    id integer PRIMARY KEY,
    name text NOT NULL,
    main integer NOT NULL REFERENCES part_types(id),
    hooks integer[] NOT NULL
);

CREATE TABLE parts (
    id SERIAL PRIMARY KEY,
    owner integer NOT NULL,
    what integer NOT NULL REFERENCES part_types(id),
    name text NOT NULL,
    vendor text NOT NULL DEFAULT '""'::text,
    model text NOT NULL DEFAULT '""'::text,
    purchase timestamp with time zone NOT NULL DEFAULT now(),
    time integer NOT NULL DEFAULT 0,
    distance integer NOT NULL DEFAULT 0,
    climb integer NOT NULL DEFAULT 0,
    descend integer NOT NULL DEFAULT 0,
    count integer NOT NULL DEFAULT 0
);

ALTER SEQUENCE parts_id_seq RESTART WITH 300;
