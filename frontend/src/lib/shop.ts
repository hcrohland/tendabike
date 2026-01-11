import { handleError, myfetch, refresh, updateSummary } from "./store";
import { mapable } from "./mapable";
import { type Part } from "./part";
import { writable } from "svelte/store";

export class Shop {
  id?: number;
  owner: number;
  owner_firstname: string;
  owner_name: string;
  name: string;
  description?: string;
  created_at: Date;

  constructor(data: any) {
    this.id = data.id;
    this.owner = data.owner;
    this.owner_firstname = data.owner_firstname || "";
    this.owner_name = data.owner_name || "";
    this.name = data.name || "";
    this.description = data.description;
    this.created_at = new Date(data.created_at);
  }

  async create() {
    return await myfetch("/api/shop", "POST", {
      name: this.name,
      description: this.description,
    })
      .then((data) => {
        shops.updateMap([data]);
        return new Shop(data);
      })
      .catch(handleError);
  }

  async update() {
    return await myfetch("/api/shop/" + this.id, "PUT", {
      name: this.name,
      description: this.description,
    })
      .then((data) => shops.updateMap([data]))
      .catch(handleError);
  }

  async delete() {
    return await myfetch("/api/shop/" + this.id, "DELETE")
      .then(() => shops.deleteItem(this.id))
      .catch(handleError);
  }

  static async registerPart(part: Part, shopid: number) {
    return await myfetch(`/api/shop/${shopid}/parts`, "POST", {
      part_id: part.id,
    })
      .then((data) => updateSummary(data))
      .catch(handleError);
  }

  static async unregisterPart(part: Part) {
    return await myfetch(`/api/shop/0/parts/${part.id}`, "DELETE")
      .then((data) => updateSummary(data))
      .catch(handleError);
  }

  async getParts(): Promise<number[]> {
    return await myfetch(`/api/shop/${this.id}/parts`, "GET").catch(
      handleError,
    );
  }

  async requestSubscription(message?: string) {
    return await myfetch("/api/shop/subscriptions", "POST", {
      shop_id: this.id,
      message: message,
    }).catch(handleError);
  }
}

// Shop mode state
export const shop = writable<Shop | undefined>(undefined);

// Exit shop mode: refresh data from backend
export async function exitShop() {
  shop.set(undefined);
  await refresh();

  // Navigate to main page
  window.location.hash = "#/cat";
}

export const shops = mapable<Shop>("id", (data) => new Shop(data));
