-- The icu event's webhook type: the row's dispatch key.
-- The `object_type` column names the object the event refers to (`ACTIVITY`);
-- this column names the webhook event type, which the queue's `process` op
-- dispatches on (`ACTIVITY_DELETED` deletes the activity; the rest import).
--
-- The backfill default is `ACTIVITY_UPLOADED`: it keeps any pre-existing
-- queued rows on the import path.
ALTER TABLE icu_events
    ADD COLUMN event_type text NOT NULL DEFAULT 'ACTIVITY_UPLOADED';

COMMENT ON COLUMN icu_events.event_type IS
    'The icu webhook event type (ACTIVITY_UPLOADED | ACTIVITY_UPDATED | ACTIVITY_DELETED); the queue dispatch key';
