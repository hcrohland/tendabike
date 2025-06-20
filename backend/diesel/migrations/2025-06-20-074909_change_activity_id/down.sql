-- -- This file should undo anything in `up.sql`
-- ALTER TABLE "activities"
--     ADD COLUMN "id" serial PRIMARY KEY,
--     DROP COLUMN "uuid";
-- CREATE TABLE new_activities(
--     id serial PRIMARY KEY,
--     user_id integer NOT NULL,
--     what integer NOT NULL,
--     name text NOT NULL,
--     start timestamp with time zone NOT NULL DEFAULT now(),
--     duration integer NOT NULL,
--     time integer,
--     distance integer,
--     climb integer,
--     descend integer,
--     energy integer,
--     gear integer,
--     utc_offset integer NOT NULL DEFAULT 0
-- );
-- ALTER SEQUENCE new_activities_id_seq
--     RESTART WITH 300;
-- INSERT INTO new_activities(user_id, what, name, start, duration, time, distance, climb, descend, energy, gear, utc_offset)
-- SELECT
--     user_id,
--     what,
--     name,
--     START,
--     duration,
--     time,
--     distance,
--     climb,
--     descend,
--     energy,
--     gear,
--     utc_offset
-- FROM
--     activities;
-- DROP TABLE activities;
-- ALTER TABLE new_activities RENAME TO activities;
ALTER TABLE "activities" RENAME COLUMN "id" TO "id2";

ALTER TABLE "activities" RENAME COLUMN "idalt" TO "id";

ALTER TABLE "activities"
    DROP CONSTRAINT "activities_pkey",
    ADD PRIMARY KEY ("id");

ALTER TABLE "activities"
    DROP COLUMN "id2";

