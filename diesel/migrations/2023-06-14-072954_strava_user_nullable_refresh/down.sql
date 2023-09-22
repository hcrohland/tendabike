-- This file should undo anything in `up.sql`
ALTER TABLE "strava_users" ALTER COLUMN "refresh_token" SET NOT NULL;
