-- Drop indexes
DROP INDEX IF EXISTS idx_shop_parts_part;
DROP INDEX IF EXISTS idx_shop_parts_shop;
DROP INDEX IF EXISTS idx_shops_owner;

-- Drop junction table
DROP TABLE IF EXISTS shop_parts;

-- Drop shops table
DROP TABLE IF EXISTS shops;
