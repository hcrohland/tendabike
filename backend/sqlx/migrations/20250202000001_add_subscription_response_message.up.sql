-- Add response_message field to store shop owner's response when approving/rejecting
ALTER TABLE shop_subscriptions
ADD COLUMN IF NOT EXISTS response_message TEXT;
