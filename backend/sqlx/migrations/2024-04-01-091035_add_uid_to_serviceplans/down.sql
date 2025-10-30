ALTER TABLE service_plans drop column uid;
ALTER TABLE service_plans
ALTER COLUMN part
SET NOT NULL;