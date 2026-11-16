-- The icu event's webhook type (issue #920): the row's dispatch key.
-- The `object_type` column names the object the event refers to (`ACTIVITY`);
-- this column names the webhook event type, which the queue's `process` op
-- dispatches on (`ACTIVITY_DELETED` deletes the activity; the rest import).
--
-- Existing rows were only ever `ACTIVITY_UPLOADED` / `ACTIVITY_UPDATED` (both
-- import), so the backfill default is `ACTIVITY_UPLOADED` — it preserves the
-- import behaviour of any row already in the queue.
ALTER TABLE icu_events
    ADD COLUMN event_type text NOT NULL DEFAULT 'ACTIVITY_UPLOADED';

COMMENT ON COLUMN icu_events.event_type IS
    'The icu webhook event type (ACTIVITY_UPLOADED | ACTIVITY_UPDATED | ACTIVITY_DELETED); the queue dispatch key';
