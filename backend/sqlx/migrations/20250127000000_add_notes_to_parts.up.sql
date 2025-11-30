-- Add notes column to parts table
ALTER TABLE parts ADD COLUMN IF NOT EXISTS notes text NOT NULL DEFAULT '';
