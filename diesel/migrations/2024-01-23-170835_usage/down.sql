-- This file should undo anything in `up.sql`
DROP TABLE "usages";

ALTER TABLE "public"."parts"
  DROP COLUMN "usage",
  ADD COLUMN "time" integer NOT NULL DEFAULT '0',
  ADD COLUMN "distance" integer NOT NULL DEFAULT '0',
  ADD COLUMN "climb" integer NOT NULL DEFAULT '0',
  ADD COLUMN "descend" integer NOT NULL DEFAULT '0',
  ADD COLUMN "count" integer NOT NULL DEFAULT '0';
