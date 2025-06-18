-- This file should undo anything in `up.sql`
DROP INDEX "source_key";
ALTER TABLE "parts" DROP COLUMN "source";
