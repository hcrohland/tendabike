-- Your SQL goes here
ALTER TABLE "strava_users"
  DROP COLUMN "access_token",
  DROP COLUMN "expires_at";