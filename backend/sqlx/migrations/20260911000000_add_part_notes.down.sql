-- Note: file notes are permanently discarded (only text notes are aggregated back).
ALTER TABLE parts ADD COLUMN notes TEXT NOT NULL DEFAULT '';

UPDATE parts p SET notes = (
    SELECT string_agg(n.name, E'\n\n')
    FROM part_notes n WHERE n.part = p.id AND n.kind = 'text'
) WHERE EXISTS (SELECT 1 FROM part_notes n WHERE n.part = p.id AND n.kind = 'text');

DROP TABLE part_notes;
