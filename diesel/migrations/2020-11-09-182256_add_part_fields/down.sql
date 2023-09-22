-- This file should undo anything in `up.sql`
ALTER TABLE "parts"
  DROP COLUMN "last_used",
  DROP COLUMN "disposed_at";
