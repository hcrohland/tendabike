-- Reverse the icu event_type column.
ALTER TABLE icu_events DROP COLUMN IF EXISTS event_type;
