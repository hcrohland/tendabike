-- Move the subscription's request/response messages onto the matching
-- notifications, then drop the columns from the subscription table.
--
-- The subscription request notification (kind = 'sub_request') carries the
-- request text; the subscription response notification (kind = 'sub_response')
-- carries the shop's reply (or the auto-approve welcome). Each is matched
-- one-to-one via the notification's subscription subject.

UPDATE notifications
SET message = s.message
FROM shop_subscriptions s
WHERE notifications.subscription_id = s.id
  AND notifications.kind = 'sub_request'
  AND s.message IS NOT NULL;

UPDATE notifications
SET message = s.response_message
FROM shop_subscriptions s
WHERE notifications.subscription_id = s.id
  AND notifications.kind = 'sub_response'
  AND s.response_message IS NOT NULL;

ALTER TABLE shop_subscriptions DROP COLUMN message;
ALTER TABLE shop_subscriptions DROP COLUMN response_message;
