-- Create garages table
CREATE TABLE IF NOT EXISTS garages (
    id SERIAL PRIMARY KEY,
    owner INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add index on owner for efficient lookups
CREATE INDEX IF NOT EXISTS idx_garages_owner ON garages(owner);

-- Create junction table for garage-part relationships
-- This allows garages to manage multiple bikes, and tracks when bikes were registered
CREATE TABLE IF NOT EXISTS garage_parts (
    garage_id INTEGER NOT NULL REFERENCES garages(id) ON DELETE CASCADE,
    part_id INTEGER NOT NULL REFERENCES parts(id) ON DELETE CASCADE,
    registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (garage_id, part_id)
);

-- Add indexes for efficient lookups in both directions
CREATE INDEX IF NOT EXISTS idx_garage_parts_garage ON garage_parts(garage_id);
CREATE INDEX IF NOT EXISTS idx_garage_parts_part ON garage_parts(part_id);
