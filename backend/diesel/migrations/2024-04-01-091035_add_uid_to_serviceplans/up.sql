ALTER TABLE "service_plans"
ADD COLUMN "uid" integer;
ALTER TABLE "service_plans"
ALTER COLUMN "part" DROP NOT NULL;