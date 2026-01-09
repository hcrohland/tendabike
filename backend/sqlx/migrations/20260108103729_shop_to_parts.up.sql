-- Add up migration script here
DROP TABLE IF EXISTS shop_parts;
ALTER TABLE parts ADD COLUMN IF NOT EXISTS shop integer;
CREATE INDEX IF NOT EXISTS parts_shop_idx ON parts(shop);

