CREATE TABLE strava_gears (
    id text PRIMARY KEY,
    tendabike_id integer NOT NULL,
    user_id integer NOT NULL REFERENCES strava_users(tendabike_id)
);
INSERT into strava_gears (id, tendabike_id, user_id)
	select source as id, id as tendabike_id, "owner" as user_id from parts where source is not null;

DROP INDEX "parts_source_key";
ALTER TABLE "parts" DROP COLUMN "source";

