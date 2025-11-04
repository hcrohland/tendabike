-- Add onboarding_status column to users table
-- This tracks the user's onboarding progress through various setup steps
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS onboarding_status integer NOT NULL DEFAULT 99;

COMMENT ON COLUMN users.onboarding_status IS 'Integer representing onboarding status. 99: Completed';

