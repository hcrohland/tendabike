CREATE TABLE attachments (
    id SERIAL PRIMARY KEY,
    part_id integer NOT NULL REFERENCES parts(id),
    hook_id integer NOT NULL REFERENCES parts(id),
    attached timestamp with time zone NOT NULL,
    detached timestamp with time zone NOT NULL DEFAULT '2999-01-01 00:00:00+01'::timestamp with time zone
);

ALTER TABLE "parts" DROP COLUMN "attached_to";
