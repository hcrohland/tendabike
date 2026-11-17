-- The `SYNC` event's fixed lower bound: the date-cursor walk
-- walks `[oldest, newest]`, where `newest` is the cursor in `event_time` and
-- `oldest` is the bound the walk stops at. `NULL` for `ACTIVITY` (webhook)
-- events, which have no walk; the bound for `SYNC` events.
ALTER TABLE icu_events
    ADD COLUMN oldest bigint;

COMMENT ON COLUMN icu_events.oldest IS
    'The SYNC backfill''s fixed lower bound (unix time); NULL for ACTIVITY events';
