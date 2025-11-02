-- Add response_message field to store garage owner's response when approving/rejecting
ALTER TABLE garage_subscriptions
ADD COLUMN IF NOT EXISTS response_message TEXT;
