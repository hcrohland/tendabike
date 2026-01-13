<script lang="ts">
  import { Input, Spinner } from "flowbite-svelte";
  import ShopList from "./ShopList.svelte";
  import { handleError, myfetch } from "../lib/store";
  import { Shop } from "../lib/shop";
  import { type ShopSubscriptionFull } from "../lib/subscription";
  import { user } from "../lib/user";

  interface Props {
    subscriptions: ShopSubscriptionFull[];
  }

  let { subscriptions }: Props = $props();

  let searchQuery = $state("");
  let searchResults = $state<Shop[]>([]);
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
      searchResults = results
        .map((g: any) => new Shop(g))
        .filter(
          (s: any) =>
            s.owner != $user!.id &&
            subscriptions.every((su) => su.shop_id != s.id),
        );
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

<!-- Divider -->
<hr class="border-gray-200 dark:border-gray-700 my-4" />

<!-- Search for shops -->
<div>
  <h3 class="mb-4 text-lg font-semibold text-gray-900 dark:text-white">
    Find Shops
  </h3>
  <div class="space-y-4">
    <div>
      <div class="flex gap-2">
        <Input
          id="search"
          type="text"
          bind:value={searchQuery}
          oninput={handleSearchInput}
          placeholder="Search for shops to request subscription"
        />
      </div>
    </div>

    {#if isSearching}
      <div class="flex justify-center py-8">
        <Spinner />
      </div>
    {:else if searchResults.length > 0}
      <ShopList shops={searchResults} />
    {:else if searchQuery.trim()}
      <p class="py-8 text-center text-gray-500 dark:text-gray-400">
        No shops found matching "{searchQuery}"
      </p>
    {/if}
  </div>
</div>
