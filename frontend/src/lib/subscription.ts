import { handleError, myfetch } from "./store";
import { mapable } from "./mapable";

export type SubscriptionStatus =
  | "pending"
  | "active"
  | "rejected"
  | "cancelled";

export class GarageSubscription {
  id?: number;
  garage_id: number;
  user_id: number;
  status: SubscriptionStatus;
  message?: string;
  created_at: Date;
  updated_at: Date;

  constructor(data: any) {
    this.id = data.id;
    this.garage_id = data.garage_id;
    this.user_id = data.user_id;
    this.status = data.status;
    this.message = data.message;
    this.created_at = new Date(data.created_at);
    this.updated_at = new Date(data.updated_at);
  }

  async approve() {
    return await myfetch(`/api/garage/subscriptions/${this.id}/approve`, "POST")
      .then((data) => subscriptions.updateMap([data]))
      .catch(handleError);
  }

  async reject() {
    return await myfetch(`/api/garage/subscriptions/${this.id}/reject`, "POST")
      .then((data) => subscriptions.updateMap([data]))
      .catch(handleError);
  }

  async cancel() {
    return await myfetch(`/api/garage/subscriptions/${this.id}`, "DELETE")
      .then(() => subscriptions.deleteItem(this.id?.toString()))
      .catch(handleError);
  }

  static async getMySubscriptions(): Promise<GarageSubscription[]> {
    return await myfetch("/api/garage/subscriptions", "GET")
      .then((data) => {
        subscriptions.setMap(data);
        return data.map((s: any) => new GarageSubscription(s));
      })
      .catch(handleError);
  }

  static async getGarageSubscriptions(
    garageId: number,
  ): Promise<GarageSubscription[]> {
    return await myfetch(`/api/garage/${garageId}/subscriptions`, "GET")
      .then((data) => data.map((s: any) => new GarageSubscription(s)))
      .catch(handleError);
  }
}

export const subscriptions = mapable<GarageSubscription>(
  "id",
  (data) => new GarageSubscription(data),
);
