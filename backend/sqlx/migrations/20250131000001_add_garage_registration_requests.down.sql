-- Drop trigger and function
DROP TRIGGER IF EXISTS trigger_update_garage_registration_request_updated_at ON garage_registration_requests;
DROP FUNCTION IF EXISTS update_garage_registration_request_updated_at();

-- Drop indexes
DROP INDEX IF EXISTS idx_garage_registration_requests_status;
DROP INDEX IF EXISTS idx_garage_registration_requests_requester;
DROP INDEX IF EXISTS idx_garage_registration_requests_part;
DROP INDEX IF EXISTS idx_garage_registration_requests_garage;

-- Drop table
DROP TABLE IF EXISTS garage_registration_requests;
