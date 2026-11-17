<script lang="ts">
  import { onMount } from "svelte";
  import { Button } from "flowbite-svelte";
  import {
    getEnabledProviders,
    loadProviders,
    providerMeta,
    loginUrl,
    type Provider,
  } from "../lib/provider";
  import ProviderMark from "./ProviderMark.svelte";
  import * as m from "../../paraglide/messages";

  // The enabled providers (issue #908): fetched on mount — the connect
  // screen is the anonymous landing page. `undefined` while the fetch is in
  // flight: no cards and no failure page until the set is known.
  onMount(() => {
    loadProviders();
  });

  const enabled = $derived(getEnabledProviders());

  function blurb(p: Provider) {
    return p === "strava" ? m.connect_blurb_strava() : m.connect_blurb_icu();
  }
</script>

<section id="connect" class="mb-8">
  <h2 class="text-xl font-bold mb-4">{m.connect_heading()}</h2>
  {#if enabled === undefined}
    <!-- The fetch is in flight: nothing to offer yet -->
  {:else if enabled.length === 0}
    <!-- No provider is enabled (issue #908): the failure page — a clear,
         non-leaky message, no provider cards, no login affordance -->
    <p class="text-text-1">{m.no_providers_configured()}</p>
  {:else}
    <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
      {#each enabled as p (p)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="rounded-2xl overflow-hidden border border-border-subtle bg-surface-1 shadow-sm cursor-pointer"
          onclick={() => (window.location.href = loginUrl(p))}
        >
          <div
            class="p-4 text-white"
            style="background:{providerMeta[p].accent}"
          >
            <div class="flex items-center gap-2">
              <ProviderMark provider={p} size="lg" colored />
              <span class="text-lg font-bold">{providerMeta[p].name}</span>
            </div>
            <p class="text-sm opacity-90 mt-1">{blurb(p)}</p>
          </div>
          <div class="p-4">
            <ul
              class="text-sm text-text-1 space-y-1 mb-4 list-disc list-inside"
            >
              <li>{m.connect_feature_activities()}</li>
              <li>{m.connect_feature_live_sync()}</li>
              <li>{m.connect_feature_reconnect()}</li>
            </ul>
            <Button
              color={p === "strava" ? "orange" : "gray"}
              class="w-full"
              onclick={() => (window.location.href = loginUrl(p))}
            >
              {m.connect_button({ name: providerMeta[p].name })}
            </Button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</section>
