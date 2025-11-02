-- Create garage subscriptions table
-- This allows users to subscribe to garages, and once subscribed,
-- they can freely register their bikes without per-bike approval
CREATE TABLE IF NOT EXISTS garage_subscriptions (
    id SERIAL PRIMARY KEY,
    garage_id INTEGER NOT NULL REFERENCES garages(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'active', 'rejected', 'cancelled')),
    message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(garage_id, user_id, status)
);

-- Add indexes for efficient lookups
CREATE INDEX IF NOT EXISTS idx_garage_subscriptions_garage ON garage_subscriptions(garage_id);
CREATE INDEX IF NOT EXISTS idx_garage_subscriptions_user ON garage_subscriptions(user_id);
CREATE INDEX IF NOT EXISTS idx_garage_subscriptions_status ON garage_subscriptions(status);

-- Add function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_garage_subscription_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Add trigger to automatically update updated_at
CREATE TRIGGER trigger_update_garage_subscription_updated_at
    BEFORE UPDATE ON garage_subscriptions
    FOR EACH ROW
    EXECUTE FUNCTION update_garage_subscription_updated_at();

-- Migrate existing approved registration requests to subscriptions
-- For each unique user-garage combination where there's an approved request,
-- create an active subscription
INSERT INTO garage_subscriptions (garage_id, user_id, status, message, created_at, updated_at)
SELECT DISTINCT ON (garage_id, requester_id)
    garage_id,
    requester_id,
    'active',
    'Migrated from approved registration request',
    MIN(created_at),
    NOW()
FROM garage_registration_requests
WHERE status = 'approved'
GROUP BY garage_id, requester_id
ON CONFLICT DO NOTHING;

-- Drop the old garage_registration_requests table as it's no longer needed
-- The new model doesn't require per-bike approval
DROP TRIGGER IF EXISTS trigger_update_garage_registration_request_updated_at ON garage_registration_requests;
DROP FUNCTION IF EXISTS update_garage_registration_request_updated_at();
DROP TABLE IF EXISTS garage_registration_requests;
