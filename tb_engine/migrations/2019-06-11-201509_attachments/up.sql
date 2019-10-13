CREATE TABLE attachments (
    part_id integer NOT NULL REFERENCES parts(id),
    hook_id integer NOT NULL REFERENCES parts(id),
    attached timestamp with time zone NOT NULL,
    detached timestamp with time zone,
    PRIMARY KEY (part_id, attached)
);

ALTER TABLE "parts" DROP COLUMN "attached_to";
