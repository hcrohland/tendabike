-- Remove notes column from parts table
ALTER TABLE parts DROP COLUMN IF EXISTS notes;
