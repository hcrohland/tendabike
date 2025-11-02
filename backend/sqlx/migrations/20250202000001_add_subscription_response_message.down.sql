-- Remove response_message field
ALTER TABLE garage_subscriptions
DROP COLUMN IF EXISTS response_message;
