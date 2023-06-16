-- This file should undo anything in `up.sql`
-- Your SQL goes here
ALTER TABLE "strava_users"
  DROP COLUMN "refresh_token";

ALTER TABLE "strava_users"
  ADD COLUMN "access_token" text DEFAULT '' NOT NULL,
  ADD COLUMN "expires_at" bigint DEFAULT 0 NOT NULL,
  ADD COLUMN "refresh_token" text;
