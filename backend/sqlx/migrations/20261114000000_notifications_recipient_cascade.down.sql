-- Revert the recipient FK to its original form (no cascade).

ALTER TABLE notifications DROP CONSTRAINT notifications_recipient_fkey;

ALTER TABLE notifications
    ADD CONSTRAINT notifications_recipient_fkey
    FOREIGN KEY (recipient) REFERENCES users(id);
