import { Shop } from "./shop";
import type { UserPublic } from "./user";

export type SubscriptionStatus =
  | "pending"
  | "active"
  | "rejected"
  | "cancelled";

export class ShopSubscription {
  id?: number;
  shop_id: number;
  user_id: number;
  status: SubscriptionStatus;
  message?: string;
  response_message?: string;
  created_at: Date;
  updated_at: Date;
  shop: Shop;
  owner: UserPublic;

  constructor(data: any) {
    this.id = data.id;
    this.shop_id = data.shop_id;
    this.user_id = data.user_id;
    this.status = data.status;
    this.message = data.message;
    this.response_message = data.response_message;
    this.created_at = new Date(data.created_at);
    this.updated_at = new Date(data.updated_at);
    this.shop = new Shop(data.shop);
    this.owner = data.owner;
  }
}
