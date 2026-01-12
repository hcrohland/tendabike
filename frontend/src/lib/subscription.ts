import { handleError, myfetch } from "./store";
import { mapable } from "./mapable";

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
  created_at: Date;
  updated_at: Date;

  constructor(data: any) {
    this.id = data.id;
    this.shop_id = data.shop_id;
    this.user_id = data.user_id;
    this.status = data.status;
    this.message = data.message;
    this.created_at = new Date(data.created_at);
    this.updated_at = new Date(data.updated_at);
  }

  async approve() {
    return await myfetch(`/api/shop/subscriptions/${this.id}/approve`, "POST")
      .then((data) => subscriptions.updateMap([data]))
      .catch(handleError);
  }

  async reject() {
    return await myfetch(`/api/shop/subscriptions/${this.id}/reject`, "POST")
      .then((data) => subscriptions.updateMap([data]))
      .catch(handleError);
  }

  async cancel() {
    return await myfetch(`/api/shop/subscriptions/${this.id}`, "DELETE")
      .then(() => subscriptions.deleteItem(this.id?.toString()))
      .catch(handleError);
  }

  static async getMySubscriptions(): Promise<ShopSubscription[]> {
    return await myfetch("/api/shop/subscriptions", "GET")
      .then((data) => {
        subscriptions.setMap(data);
        return data.map((s: any) => new ShopSubscription(s));
      })
      .catch(handleError);
  }

  static async getShopSubscriptions(
    shopId: number,
  ): Promise<ShopSubscription[]> {
    return await myfetch(`/api/shop/${shopId}/subscriptions`, "GET")
      .then((data) => data.map((s: any) => new ShopSubscription(s)))
      .catch(handleError);
  }
}

export const subscriptions = mapable<ShopSubscription>(
  "id",
  (data) => new ShopSubscription(data),
);
export interface ShopSubscriptionFull {
  id: number;
  shop_id: number;
  shop_name?: string;
  shop_owner_firstname?: string;
  shop_owner_name?: string;
  user_id: number;
  status: "pending" | "active" | "rejected" | "cancelled";
  message?: string;
  response_message?: string;
  created_at: string;
  updated_at: string;
}
