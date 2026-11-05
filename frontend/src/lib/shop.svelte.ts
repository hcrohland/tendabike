import { goto } from "$app/navigation";
import { handleError, myfetch } from "./utils";
import { refresh, getUser } from "./user";
import { mapableState } from "./mapable.svelte";
import { type Part } from "./part";

export class Shop {
  id?: number;
  owner: number;
  name: string;
  description?: string;
  auto_approve: boolean;
  created_at: Date;

  constructor(data: any) {
    this.id = data.id;
    this.owner = data.owner;
    this.name = data.name || "";
    this.description = data.description;
    this.created_at = new Date(data.created_at);
    this.auto_approve = data.auto_approve;
  }

  /// The writes answer 204 (201 for creates): the change rides the stream
  /// frame, so the handlers only await the write.
  async create() {
    return await myfetch("/api/shop", "POST", {
      name: this.name,
      description: this.description,
      auto_approve: this.auto_approve,
    }).catch(handleError);
  }

  async update() {
    return await myfetch("/api/shop/" + this.id, "PUT", {
      name: this.name,
      description: this.description,
      auto_approve: this.auto_approve,
    }).catch(handleError);
  }

  async delete() {
    return await myfetch("/api/shop/" + this.id, "DELETE").catch(handleError);
  }

  static async registerPart(part: Part, shopid: number) {
    return await myfetch(`/api/shop/${shopid}/parts`, "POST", {
      part_id: part.id,
    }).catch(handleError);
  }

  static async unregisterPart(part: Part) {
    if (part.shop == null) return;
    return await myfetch(
      `/api/shop/${part.shop}/parts/${part.id}`,
      "DELETE",
    ).catch(handleError);
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

/**
 * The current shop (shop mode), as a Svelte 5 state object. The value is
 * nullable and replaced wholesale, which a `.svelte.ts` module may not
 * export directly (Svelte's `state_invalid_export` rule), so the state
 * lives in the module and is read via `getShop` and replaced via
 * `setShop`. Reads register dependencies at the enclosing reactive call
 * site.
 */
let shop = $state<Shop | undefined>(undefined);

export function getShop(): Shop | undefined {
  return shop;
}

/** Internal: replace the active shop. Use `enterShop`/`exitShop` in
 * application code. */
export function setShop(value: Shop | undefined) {
  shop = value;
}

// Enter shop mode: replaces stores with shop-specific data. The refresh's
// shop scope rides the `X-Active-Shop` header the fetch layer injects from
// the tab's shop (ADR-0009).
export async function enterShop(shop: Shop) {
  setShop(shop);
  if (shop.owner == getUser()?.id) await refresh();
  goto("/cat");
}

// Exit shop mode: refresh data from backend
export async function exitShop() {
  setShop(undefined);
  await refresh();

  // Navigate to main page
  goto("/cat");
}

export const shops = mapableState("id", (data) => new Shop(data));
