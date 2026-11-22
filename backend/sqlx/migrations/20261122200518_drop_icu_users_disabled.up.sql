-- Drop the icu_users.disabled column: vestigial. The disable signal is the
-- cleared access token (IcuUser::disabled derives it from
-- `access_token IS NULL`); the column was only ever written `false` and never
-- read for the disabled determination.
ALTER TABLE icu_users DROP COLUMN disabled;
