-- Remove onboarding_status column from users table

ALTER TABLE users
DROP COLUMN IF EXISTS onboarding_status;
