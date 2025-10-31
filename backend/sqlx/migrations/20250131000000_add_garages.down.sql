-- Drop indexes
DROP INDEX IF EXISTS idx_garage_parts_part;
DROP INDEX IF EXISTS idx_garage_parts_garage;
DROP INDEX IF EXISTS idx_garages_owner;

-- Drop junction table
DROP TABLE IF EXISTS garage_parts;

-- Drop garages table
DROP TABLE IF EXISTS garages;
