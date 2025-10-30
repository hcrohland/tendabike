-- Your SQL goes here
ALTER TABLE "parts" ADD COLUMN "source" text;
CREATE INDEX "source_key" ON "parts"("source");

update parts
	SET source = res.id
	 from (select * from strava_gears) as res
	 where parts.id = res.tendabike_id
	;   