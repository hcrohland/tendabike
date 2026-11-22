export const ssr = false;

import { redirect } from "@sveltejs/kit";
import { getTypes } from "../lib/types.svelte";
import { initData, getUser } from "../lib/user.svelte";

export async function load({ url }) {
  // Step 1: load types (hard prerequisite)
  await getTypes();
  // Step 2: load user + summary
  await initData();
  // Anonymous / 401: redirect to about (skip if already on a public page —
  // /about is the guide entrance, and its /about/[topic] pages are public too)
  if (
    !getUser() &&
    url.pathname !== "/about" &&
    !url.pathname.startsWith("/about/") &&
    url.pathname !== "/data-protection"
  ) {
    throw redirect(302, "/about");
  }
}
