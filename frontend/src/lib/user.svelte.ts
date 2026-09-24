import type { User } from "./user";

/**
 * The current user, as a Svelte 5 state object (the store-to-state
 * migration of the former svelte/store writable in user.ts). The value is
 * nullable and replaced wholesale, which a `.svelte.ts` module may not
 * export directly (Svelte's `state_invalid_export` rule), so the state
 * lives in the module and is read via `getUser` and replaced via
 * `setUser`. Reads register dependencies at the enclosing reactive call
 * site; readers use `getUser()` where they formerly used `$user` /
 * `get(user)`.
 */
let user = $state<User | undefined>(undefined);

export function getUser(): User | undefined {
  return user;
}

/** Replace the current user; `undefined` logs out. */
export function setUser(value: User | undefined) {
  user = value;
}
