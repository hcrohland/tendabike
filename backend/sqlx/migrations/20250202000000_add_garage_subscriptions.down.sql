
-- Drop the shop_subscriptions table
DROP TRIGGER IF EXISTS trigger_update_shop_subscription_updated_at ON shop_subscriptions;
DROP FUNCTION IF EXISTS update_shop_subscription_updated_at();
DROP TABLE IF EXISTS shop_subscriptions;
