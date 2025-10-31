-- Create garage registration requests table
-- This allows users to request to register their bikes to garages
-- and garage owners to approve/reject those requests
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

-- Add indexes for efficient lookups
CREATE INDEX IF NOT EXISTS idx_garage_registration_requests_garage ON garage_registration_requests(garage_id);
CREATE INDEX IF NOT EXISTS idx_garage_registration_requests_part ON garage_registration_requests(part_id);
CREATE INDEX IF NOT EXISTS idx_garage_registration_requests_requester ON garage_registration_requests(requester_id);
CREATE INDEX IF NOT EXISTS idx_garage_registration_requests_status ON garage_registration_requests(status);

-- Add function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_garage_registration_request_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Add trigger to automatically update updated_at
CREATE TRIGGER trigger_update_garage_registration_request_updated_at
    BEFORE UPDATE ON garage_registration_requests
    FOR EACH ROW
    EXECUTE FUNCTION update_garage_registration_request_updated_at();
