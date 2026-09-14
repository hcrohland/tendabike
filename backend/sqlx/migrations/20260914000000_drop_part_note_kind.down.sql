ALTER TABLE part_notes ADD COLUMN kind TEXT NOT NULL DEFAULT 'text';
UPDATE part_notes SET kind = 'file' WHERE mime IS NOT NULL;
