-- Create shop subscriptions table
-- This allows users to subscribe to shops, and once subscribed,
-- they can freely register their bikes without per-bike approval
CREATE TABLE IF NOT EXISTS shop_subscriptions (
    id SERIAL PRIMARY KEY,
    shop_id INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'active', 'rejected', 'cancelled')),
    message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(shop_id, user_id, status)
);

-- Add indexes for efficient lookups
CREATE INDEX IF NOT EXISTS idx_shop_subscriptions_shop ON shop_subscriptions(shop_id);
CREATE INDEX IF NOT EXISTS idx_shop_subscriptions_user ON shop_subscriptions(user_id);
CREATE INDEX IF NOT EXISTS idx_shop_subscriptions_status ON shop_subscriptions(status);

-- Add function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_shop_subscription_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Add trigger to automatically update updated_at
CREATE TRIGGER trigger_update_shop_subscription_updated_at
    BEFORE UPDATE ON shop_subscriptions
    FOR EACH ROW
    EXECUTE FUNCTION update_shop_subscription_updated_at();
