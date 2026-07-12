<script lang="ts">
  import { Input, Spinner } from "flowbite-svelte";
  import * as m from "../../paraglide/messages";

  import ShopList from "./ShopList.svelte";
  import { handleError, myfetch } from "../lib/store";
  import { Shop } from "../lib/shop";
  import { type ShopSubscription } from "../lib/subscription";
  import { user, type UserPublic } from "../lib/user";
  import { type Map } from "../lib/mapable";

  interface Props {
    subscriptions: ShopSubscription[];
  }

  let { subscriptions }: Props = $props();

  let searchQuery = $state("");
  let searchResults = $state<Shop[]>([]);
  let users: Map<UserPublic> = $state({});

  let isSearching = $state(false);

  async function performSearch() {
    if (!searchQuery.trim()) {
      searchResults = [];
      return;
    }

    isSearching = true;
    try {
      const results = await myfetch(
        `/api/shop/search?q=${encodeURIComponent(searchQuery)}`,
        "GET",
      );
      searchResults = results[0]
        .map((g: any) => new Shop(g))
        .filter(
          (s: any) =>
            s.owner != $user!.id &&
            subscriptions.every((su) => su.shop_id != s.id),
        );
      results[1].map((u: UserPublic) => (users[u.id] = u));
    } catch (error) {
      handleError(error as Error);
    } finally {
      isSearching = false;
    }
  }

  function handleSearchInput() {
    // Debounce search
    const timeoutId = setTimeout(() => {
      performSearch();
    }, 300);
    return () => clearTimeout(timeoutId);
  }
</script>

<hr class="my-4 border-gray-200 dark:border-gray-700" />

<div>
  <h3 class="mb-4 text-lg font-semibold text-gray-900 dark:text-white">
    {m.shop_search_header()}
  </h3>

  <div class="space-y-4">
    <div class="flex gap-2">
      <Input
        id="search"
        type="text"
        bind:value={searchQuery}
        oninput={handleSearchInput}
        placeholder={m.shop_search_placeholder()}
      />
    </div>

    {#if isSearching}
      <div class="flex justify-center py-8">
        <Spinner />
      </div>
    {:else if searchResults.length > 0}
      <ShopList shops={searchResults} {users} />
    {:else if searchQuery.trim()}
      <p class="py-8 text-center text-text-1">
        {m.shop_search_none({ query: searchQuery })}
      </p>
    {/if}
  </div>
</div>
