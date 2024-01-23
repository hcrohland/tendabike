CREATE TABLE usages (
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    time integer,
    distance integer,
    climb integer,
    descend integer,
    power integer,
    count integer
);

ALTER TABLE "public"."parts"
  DROP COLUMN "time",
  DROP COLUMN "distance",
  DROP COLUMN "climb",
  DROP COLUMN "descend",
  DROP COLUMN "count",
  ADD COLUMN "usage" uuid;
