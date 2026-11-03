-- Restore the subscription's request/response message columns. The message
-- data moved onto the notifications is not recovered.
ALTER TABLE shop_subscriptions ADD COLUMN message TEXT;
ALTER TABLE shop_subscriptions ADD COLUMN response_message TEXT;
