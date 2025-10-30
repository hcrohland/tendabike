CREATE TABLE "services" (
    "id" uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    "part_id" integer NOT NULL,
    "time" timestamp with time zone NOT NULL,
    "redone" timestamp with time zone NOT NULL,
    "name" text NOT NULL,
    "notes" text NOT NULL DEFAULT '',
    "usage" uuid NOT NULL
);
