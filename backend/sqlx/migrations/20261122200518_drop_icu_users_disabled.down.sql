-- Re-add the icu_users.disabled column (the pre-drop schema).
ALTER TABLE icu_users ADD COLUMN disabled boolean NOT NULL DEFAULT false;
