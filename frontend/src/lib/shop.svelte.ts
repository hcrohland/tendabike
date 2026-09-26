import type { Shop } from "./shop";

/**
 * The current shop (shop mode), as a Svelte 5 state object (the
 * store-to-state migration of the former svelte/store writable in
 * shop.ts). The value is nullable and replaced wholesale, which a
 * `.svelte.ts` module may not export directly (Svelte's
 * `state_invalid_export` rule), so the state lives in the module and is
 * read via `getShop` and replaced via `setShop`. Reads register
 * dependencies at the enclosing reactive call site; readers use
 * `getShop()` where they formerly used `$shop` / `get(shop)`.
 */
let shop = $state<Shop | undefined>(undefined);

export function getShop(): Shop | undefined {
  return shop;
}

/** Enter shop mode with `value`; `undefined` exits it. */
export function setShop(value: Shop | undefined) {
  shop = value;
}
