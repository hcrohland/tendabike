-- This file should undo anything in `up.sql`
CREATE TABLE part_types (
    id integer PRIMARY KEY,
    name text NOT NULL,
    main integer NOT NULL,
    hooks integer[] NOT NULL,
    "order" integer NOT NULL DEFAULT 9999,
    "group" text
);

CREATE TABLE activity_types (
    id SERIAL PRIMARY KEY,
    name text NOT NULL,
    gear integer NOT NULL 
);



