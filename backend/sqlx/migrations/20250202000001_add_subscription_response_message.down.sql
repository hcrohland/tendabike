-- Remove response_message field
ALTER TABLE shop_subscriptions
DROP COLUMN IF EXISTS response_message;
