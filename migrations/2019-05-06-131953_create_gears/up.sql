CREATE TABLE part_types (
    id SERIAL PRIMARY KEY,
    name text NOT NULL,
    parts integer[] NOT NULL DEFAULT '{}'::integer[],
    main BOOLEAN NOT NULL DEFAULT FALSE
);


CREATE TABLE "parts" (
    "id" serial PRIMARY KEY,
    "owner" integer NOT NULL,
    "what" integer NOT NULL REFERENCES part_types(id),
    "name" text NOT NULL,
    "vendor" text NOT NULL DEFAULT '""',
    "model" text NOT NULL DEFAULT '""',
    "purchase" timestamptz NOT NULL DEFAULT now(),
    "time" integer NOT NULL DEFAULT '0',
    "distance" integer NOT NULL DEFAULT '0',
    "climb" integer NOT NULL DEFAULT '0',
    "descend" integer NOT NULL DEFAULT '0',
    "attached_to" integer
);
