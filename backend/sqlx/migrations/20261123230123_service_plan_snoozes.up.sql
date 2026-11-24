-- The plan's accumulated snooze fractions: a map from part id
-- to the accumulated non-negative fraction, owned by the plan.
ALTER TABLE service_plans
ADD COLUMN snoozes jsonb NOT NULL DEFAULT '{}'::jsonb;

COMMENT ON COLUMN service_plans.snoozes IS 'Map of part id to accumulated snooze fraction';
