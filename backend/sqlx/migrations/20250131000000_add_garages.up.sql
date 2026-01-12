-- Create shops table
CREATE TABLE IF NOT EXISTS shops (
    id SERIAL PRIMARY KEY,
    owner INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add index on owner for efficient lookups
CREATE INDEX IF NOT EXISTS idx_shops_owner ON shops(owner);

-- Create junction table for shop-part relationships
-- This allows shops to manage multiple bikes, and tracks when bikes were registered
CREATE TABLE IF NOT EXISTS shop_parts (
    shop_id INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    part_id INTEGER NOT NULL REFERENCES parts(id) ON DELETE CASCADE,
    registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (shop_id, part_id)
);

-- Add indexes for efficient lookups in both directions
CREATE INDEX IF NOT EXISTS idx_shop_parts_shop ON shop_parts(shop_id);
CREATE INDEX IF NOT EXISTS idx_shop_parts_part ON shop_parts(part_id);
