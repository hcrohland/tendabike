CREATE TABLE part_notes (
    id SERIAL PRIMARY KEY,
    part INTEGER NOT NULL REFERENCES parts(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    mime TEXT,
    size BIGINT,
    data BYTEA,
    created TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_part_notes_part ON part_notes (part);

INSERT INTO part_notes (part, kind, name, created)
SELECT id, 'text', notes, now()
FROM parts
WHERE notes IS NOT NULL AND notes != '';

ALTER TABLE parts DROP COLUMN notes;
