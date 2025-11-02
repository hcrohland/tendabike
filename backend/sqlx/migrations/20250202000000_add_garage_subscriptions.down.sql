-- Recreate the old garage_registration_requests table
CREATE TABLE IF NOT EXISTS garage_registration_requests (
    id SERIAL PRIMARY KEY,
    garage_id INTEGER NOT NULL REFERENCES garages(id) ON DELETE CASCADE,
    part_id INTEGER NOT NULL REFERENCES parts(id) ON DELETE CASCADE,
    requester_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected')),
    message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(garage_id, part_id, status)
);

CREATE INDEX IF NOT EXISTS idx_garage_registration_requests_garage ON garage_registration_requests(garage_id);
CREATE INDEX IF NOT EXISTS idx_garage_registration_requests_part ON garage_registration_requests(part_id);
CREATE INDEX IF NOT EXISTS idx_garage_registration_requests_requester ON garage_registration_requests(requester_id);
CREATE INDEX IF NOT EXISTS idx_garage_registration_requests_status ON garage_registration_requests(status);

CREATE OR REPLACE FUNCTION update_garage_registration_request_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_garage_registration_request_updated_at
    BEFORE UPDATE ON garage_registration_requests
    FOR EACH ROW
    EXECUTE FUNCTION update_garage_registration_request_updated_at();

-- Drop the garage_subscriptions table
DROP TRIGGER IF EXISTS trigger_update_garage_subscription_updated_at ON garage_subscriptions;
DROP FUNCTION IF EXISTS update_garage_subscription_updated_at();
DROP TABLE IF EXISTS garage_subscriptions;
