-- ALTER TABLE "activities"
--     ADD COLUMN "uuid" uuid DEFAULT gen_random_uuid() PRIMARY KEY,
--     DROP COLUMN "id";
ALTER TABLE "activities"
    ADD COLUMN "id2" bigint;

UPDATE
    activities
SET
    id2 = res.id
FROM (
    SELECT
        *
    FROM
        strava_activities) AS res
WHERE
    activities.id = res.tendabike_id;

ALTER TABLE "activities"
    DROP CONSTRAINT "activities_pkey",
    ADD PRIMARY KEY ("id2");

ALTER TABLE "activities"
    DROP COLUMN "id";

ALTER TABLE "activities" RENAME COLUMN "id2" TO "id";

