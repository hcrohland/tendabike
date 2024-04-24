ALTER TABLE "attachments"
    add column "count" integer NOT NULL DEFAULT 0,
    add column time integer NOT NULL DEFAULT 0,
    add column distance integer NOT NULL DEFAULT 0,
    add column climb integer NOT NULL DEFAULT 0,
    add column descend integer NOT NULL DEFAULT 0;