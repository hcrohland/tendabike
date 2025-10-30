-- TendaBike Database Schema - Initial Migration
-- Creates all tables if they do not exist
-- Enable UUID extension for UUID generation
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ============================================================================
-- USERS TABLE
-- ============================================================================
CREATE TABLE IF NOT EXISTS users(
    id serial PRIMARY KEY,
    name text NOT NULL,
    firstname text NOT NULL,
    is_admin boolean NOT NULL,
    avatar text
);

-- ============================================================================
-- USAGES TABLE
-- ============================================================================
-- Must be created before parts, attachments, and services (FK dependencies)
CREATE TABLE IF NOT EXISTS usages(
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    time integer,
    distance integer,
    climb integer,
    descend integer,
    energy integer NOT NULL DEFAULT 0,
    count integer DEFAULT 0
);

-- ============================================================================
-- PARTS TABLE
-- ============================================================================
CREATE TABLE IF NOT EXISTS parts(
    id serial PRIMARY KEY,
    owner integer NOT NULL,
    what integer NOT NULL,
    name text NOT NULL,
    vendor text NOT NULL DEFAULT '""'::text,
    model text NOT NULL DEFAULT '""'::text,
    purchase timestamp with time zone NOT NULL DEFAULT now(),
    last_used timestamp with time zone NOT NULL DEFAULT '1970-01-01 01:00:00+01'::timestamp with time zone,
    disposed_at timestamp with time zone,
    usage uuid NOT NULL DEFAULT gen_random_uuid(),
    source text
);

-- Create index for efficient lookup by external source ID
CREATE INDEX IF NOT EXISTS parts_source_idx ON parts(source)
WHERE
    source IS NOT NULL;

-- Set starting sequence for parts (if not already set)
DO $$
BEGIN
    IF NOT EXISTS(
        SELECT
            1
        FROM
            parts) THEN
    ALTER SEQUENCE parts_id_seq
        RESTART WITH 300;
END IF;
END
$$;

-- ============================================================================
-- ACTIVITIES TABLE
-- ============================================================================
CREATE TABLE IF NOT EXISTS activities(
    user_id integer NOT NULL DEFAULT 2,
    what integer NOT NULL,
    name text NOT NULL,
    start timestamp with time zone NOT NULL DEFAULT now(),
    duration integer NOT NULL,
    time integer,
    distance integer,
    climb integer,
    descend integer,
    energy integer,
    gear integer,
    utc_offset integer NOT NULL DEFAULT 0,
    id bigint PRIMARY KEY,
    device_name text,
    external_id text
);

-- Create indexes for common query patterns
CREATE INDEX IF NOT EXISTS activities_user_id_idx ON activities(user_id);

CREATE INDEX IF NOT EXISTS activities_gear_idx ON activities(gear)
WHERE
    gear IS NOT NULL;

CREATE INDEX IF NOT EXISTS activities_start_idx ON activities(START);

CREATE INDEX IF NOT EXISTS activities_external_id_idx ON activities(external_id)
WHERE
    external_id IS NOT NULL;

-- ============================================================================
-- ATTACHMENTS TABLE
-- ============================================================================
CREATE TABLE IF NOT EXISTS attachments(
    part_id integer REFERENCES parts(id),
    attached timestamp with time zone,
    gear integer NOT NULL REFERENCES parts(id),
    hook integer NOT NULL,
    detached timestamp with time zone NOT NULL,
    usage uuid NOT NULL DEFAULT gen_random_uuid(),
    CONSTRAINT attachments2_pkey PRIMARY KEY (part_id, attached)
);

-- Create indexes for common query patterns
CREATE INDEX IF NOT EXISTS attachments_gear_idx ON attachments(gear);

CREATE INDEX IF NOT EXISTS attachments_time_range_idx ON attachments(attached, detached);

-- ============================================================================
-- SERVICES TABLE
-- ============================================================================
CREATE TABLE IF NOT EXISTS services(
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    part_id integer NOT NULL,
    time timestamp with time zone NOT NULL,
    redone timestamp with time zone NOT NULL,
    name text NOT NULL,
    notes text NOT NULL DEFAULT ''::text,
    usage uuid NOT NULL,
    successor uuid,
    plans uuid[] DEFAULT ARRAY[]::uuid[]
);

-- Create indexes for common query patterns
CREATE INDEX IF NOT EXISTS services_part_id_idx ON services(part_id);

CREATE INDEX IF NOT EXISTS services_time_idx ON services(time);

-- ============================================================================
-- SERVICE_PLANS TABLE
-- ============================================================================
CREATE TABLE IF NOT EXISTS service_plans(
    id uuid PRIMARY KEY,
    part integer,
    what integer NOT NULL,
    hook integer,
    name text NOT NULL,
    days integer,
    hours integer,
    km integer,
    climb integer,
    descend integer,
    rides integer,
    uid integer,
    energy integer
);

-- Create indexes for common query patterns
CREATE INDEX IF NOT EXISTS service_plans_part_idx ON service_plans(part)
WHERE
    part IS NOT NULL;

CREATE INDEX IF NOT EXISTS service_plans_uid_idx ON service_plans(uid)
WHERE
    uid IS NOT NULL;

-- ============================================================================
-- STRAVA_USERS TABLE
-- ============================================================================
-- Note: id is the Strava user ID (provided by Strava API), NOT auto-generated
CREATE TABLE IF NOT EXISTS strava_users(
    id integer PRIMARY KEY,
    tendabike_id integer NOT NULL UNIQUE,
    refresh_token text
);

COMMENT ON COLUMN strava_users.id IS 'Strava user ID provided by Strava API (not auto-generated)';

-- ============================================================================
-- STRAVA_EVENTS TABLE
-- ============================================================================
CREATE TABLE IF NOT EXISTS strava_events(
    id serial PRIMARY KEY,
    object_type text NOT NULL,
    object_id bigint NOT NULL,
    aspect_type text NOT NULL,
    updates text,
    owner_id integer NOT NULL,
    subscription_id integer NOT NULL,
    event_time bigint NOT NULL
);

-- Create indexes for common query patterns
CREATE INDEX IF NOT EXISTS strava_events_owner_id_idx ON strava_events(owner_id);

CREATE INDEX IF NOT EXISTS strava_events_object_idx ON strava_events(object_id, owner_id);

CREATE INDEX IF NOT EXISTS strava_events_time_idx ON strava_events(event_time);

-- ============================================================================
-- COMMENTS
-- ============================================================================
COMMENT ON TABLE users IS 'TendaBike user accounts';

COMMENT ON TABLE activities IS 'Physical activities (rides, runs, etc.) with metrics';

COMMENT ON TABLE parts IS 'Bike components and gear';

COMMENT ON TABLE attachments IS 'History of part attachments to bikes over time';

COMMENT ON TABLE usages IS 'Aggregated usage metrics for parts and services';

COMMENT ON TABLE services IS 'Maintenance and service records';

COMMENT ON TABLE service_plans IS 'Scheduled maintenance plans and intervals';

COMMENT ON TABLE strava_users IS 'Strava integration user mappings';

COMMENT ON TABLE strava_events IS 'Strava webhook events queue';

COMMENT ON COLUMN activities.utc_offset IS 'UTC offset in seconds to preserve original timezone';

COMMENT ON COLUMN activities.external_id IS 'External service identifier (e.g., Strava activity ID)';

COMMENT ON COLUMN parts.source IS 'External source identifier (e.g., Strava gear ID)';

COMMENT ON COLUMN parts.usage IS 'Aggregate usage metrics for this part (UUID reference, no FK constraint)';

COMMENT ON COLUMN attachments.hook IS 'Attachment point/hook type identifier';

COMMENT ON COLUMN services.plans IS 'Array of service plan IDs this service fulfills';

COMMENT ON COLUMN services.successor IS 'Next service in the chain (for service history)';

COMMENT ON COLUMN strava_events.updates IS 'JSON-serialized event update data';

