-- TendaBike Database Schema - Down Migration
-- Drops all tables in reverse dependency order

-- Drop tables that reference other tables first
DROP TABLE IF EXISTS strava_events CASCADE;
DROP TABLE IF EXISTS strava_users CASCADE;
DROP TABLE IF EXISTS service_plans CASCADE;
DROP TABLE IF EXISTS services CASCADE;
DROP TABLE IF EXISTS attachments CASCADE;
DROP TABLE IF EXISTS activities CASCADE;
DROP TABLE IF EXISTS parts CASCADE;
DROP TABLE IF EXISTS usages CASCADE;
DROP TABLE IF EXISTS users CASCADE;

-- Optionally drop the extension (commented out by default to avoid affecting other databases)
-- DROP EXTENSION IF EXISTS pgcrypto;
