-- A user's notifications are deleted with the user: the recipient FK
-- gains the same ON DELETE CASCADE its two siblings carry
-- (garages.owner, garage_subscriptions.user_id).

ALTER TABLE notifications DROP CONSTRAINT notifications_recipient_fkey;

ALTER TABLE notifications
    ADD CONSTRAINT notifications_recipient_fkey
    FOREIGN KEY (recipient) REFERENCES users(id) ON DELETE CASCADE;
