-- Reverse the icu event_type column (issue #920).
ALTER TABLE icu_events DROP COLUMN IF EXISTS event_type;
