-- Restore the users.provider column (the reverse of the drop in
-- 20261114000000_drop_users_provider.up.sql). Backfilled from the
-- provider-internal user data: the default is 'strava' (the historical
-- default), and the icu users' rows are set to 'icu'.
ALTER TABLE users
    ADD COLUMN provider text NOT NULL DEFAULT 'strava';

UPDATE users u
    SET provider = 'icu'
    WHERE EXISTS (SELECT 1 FROM icu_users i WHERE i.tbid = u.id);

COMMENT ON COLUMN users.provider IS 'The activity provider (strava | icu); set at signup, immutable';
