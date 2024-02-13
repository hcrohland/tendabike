CREATE TABLE service_plans (
    "id" uuid PRIMARY KEY,
    "part" integer NOT NULL,
    "what" integer NOT NULL,
    "hook" integer,
    "name" text NOT NULL,
    "days" integer,
    "time" integer,
    "distance" integer,
    "climb" integer,
    "descend" integer,
    "rides" integer
);