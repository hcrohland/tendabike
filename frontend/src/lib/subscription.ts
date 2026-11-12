import { mapableState } from "./mapable.svelte";
import { Shop } from "./shop.svelte";
import type { UserPublic } from "./user";

export type SubscriptionStatus =
  "pending" | "active" | "rejected" | "cancelled";

export class ShopSubscription {
  id?: number;
  shop_id: number;
  user_id: number;
  status: SubscriptionStatus;
  created_at: Date;
  updated_at: Date;
  shop: Shop;
  owner: UserPublic;

  constructor(data: any) {
    this.id = data.id;
    this.shop_id = data.shop_id;
    this.user_id = data.user_id;
    this.status = data.status;
    this.created_at = new Date(data.created_at);
    this.updated_at = new Date(data.updated_at);
    this.shop = new Shop(data.shop);
    this.owner = data.owner;
  }
}

export const subscriptions = mapableState(
  "id",
  (data) => new ShopSubscription(data),
);
