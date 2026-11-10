-- Reverse the intervals.icu schema (spec #763).
DROP TABLE IF EXISTS icu_events;
DROP TABLE IF EXISTS icu_users;
ALTER TABLE users DROP COLUMN IF EXISTS provider;
