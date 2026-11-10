<script lang="ts">
  import { Button } from "flowbite-svelte";
  import {
    PROVIDERS,
    providerMeta,
    loginUrl,
    type Provider,
  } from "../lib/provider";
  import ProviderMark from "./ProviderMark.svelte";
  import * as m from "../../paraglide/messages";

  function blurb(p: Provider) {
    return p === "strava" ? m.connect_blurb_strava() : m.connect_blurb_icu();
  }
</script>

<section id="connect" class="mb-8">
  <h2 class="text-xl font-bold mb-4">{m.connect_heading()}</h2>
  <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
    {#each PROVIDERS as p (p)}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="rounded-2xl overflow-hidden border border-border-subtle bg-surface-1 shadow-sm cursor-pointer"
        onclick={() => (window.location.href = loginUrl(p))}
      >
        <div class="p-4 text-white" style="background:{providerMeta[p].accent}">
          <div class="flex items-center gap-2">
            <ProviderMark provider={p} size="lg" colored />
            <span class="text-lg font-bold">{providerMeta[p].name}</span>
          </div>
          <p class="text-sm opacity-90 mt-1">{blurb(p)}</p>
        </div>
        <div class="p-4">
          <ul class="text-sm text-text-1 space-y-1 mb-4 list-disc list-inside">
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
</section>
