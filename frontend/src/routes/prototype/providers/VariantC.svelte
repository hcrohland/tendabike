<!-- PROTOTYPE (throwaway) — Variant C: "Provider identity".
     The provider is a first-class visual identity: hero cards on connect,
     branded onboarding, prominent badges on cards + header, partner headline. -->
<script lang="ts">
  import { Button, Badge } from "flowbite-svelte";
  import Chip from "../../../Widgets/Chip.svelte";
  import Section from "./Section.svelte";
  import {
    mockActivities,
    mockGear,
    providerOfActivityId,
    providerMeta,
    type Provider,
  } from "./mock";

  function accentBg(p: Provider) {
    return p === "strava" ? "#fc4c02" : "#18181b";
  }
</script>

<!-- ① CONNECT: hero feature cards (landing-first) -->
<div
  class="rounded-lg border border-border-subtle bg-surface-1 p-3 flex items-center justify-between"
>
  <div class="flex items-center gap-2">
    <img src="/favicon.png" alt="TendaBike" class="rounded-circle h-8" />
    <span class="font-semibold">Tend a Bike</span>
  </div>
  <span class="text-xs text-text-1">pick your source to begin</span>
</div>

<Section n="①" title="Connect — two hero feature cards" />
<div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
  {#each Object.entries(providerMeta) as [key, meta] (key)}
    {@const p = key as Provider}
    <div
      class="rounded-2xl overflow-hidden border border-border-subtle bg-surface-1 shadow-sm"
    >
      <div class="p-4 text-white" style="background:{accentBg(p)}">
        <div class="flex items-center gap-2">
          {#if p === "strava"}
            <img src="/strava.png" alt="" class="h-7 w-7" />
          {:else}
            <span
              class="inline-flex items-center justify-center h-6 min-w-6 px-1.5 rounded bg-white/20 text-white text-[11px] font-bold"
              >icu</span
            >
          {/if}
          <span class="text-lg font-bold">{meta.name}</span>
        </div>
        <p class="text-sm opacity-90 mt-1">{meta.blurb}</p>
      </div>
      <div class="p-4">
        <ul class="text-sm text-text-1 space-y-1 mb-4 list-disc list-inside">
          <li>Activities &amp; gear</li>
          <li>Live sync</li>
          <li>One tap to reconnect</li>
        </ul>
        <Button color={p === "strava" ? "orange" : "gray"} class="w-full">
          Connect {meta.name}
        </Button>
      </div>
    </div>
  {/each}
</div>

<Section n="②" title="Onboarding — branded with the provider's accent" />
<div
  class="rounded-2xl border-2 overflow-hidden"
  style="border-color:{providerMeta.strava.accent}"
>
  <div
    class="p-3 text-white flex items-center gap-2"
    style="background:{providerMeta.strava.accent}"
  >
    <img src="/strava.png" alt="" class="h-6 w-6" />
    <span class="font-semibold">Import from Strava</span>
  </div>
  <div class="p-4 text-center bg-surface-1">
    <h3 class="mb-2 text-lg font-semibold">Welcome to TendaBike!</h3>
    <p class="mb-4 text-left text-sm">
      We'll pull your Strava activities and bikes so TendaBike can track their
      wear.
    </p>
    <div class="flex justify-center gap-3">
      <Button color="orange">Import Activities</Button>
      <Button color="alternative">Skip for Now</Button>
    </div>
  </div>
</div>

<Section n="③" title="View-on links — a prominent provider badge" />
<!-- connected header with a provider badge, to show identity "throughout" -->
<div
  class="rounded-lg border border-border-subtle bg-surface-1 p-3 flex items-center justify-between mb-3"
>
  <span class="text-sm text-text-1">Connected header</span>
  <div class="flex items-center gap-2">
    <Badge color="orange">
      <img src="/strava.png" alt="" class="inline h-4 w-4" /> Strava
    </Badge>
    <span
      class="inline-block h-7 w-7 rounded-full bg-neutral-300 dark:bg-neutral-600"
      aria-label="avatar"
    ></span>
  </div>
</div>
<div class="space-y-2">
  {#each mockActivities as a}
    {@const prov = providerOfActivityId(a.id)}
    <div
      class="rounded-lg border border-border-subtle bg-surface-1 p-3 flex items-center justify-between gap-2"
    >
      <div class="min-w-0">
        <span class="text-sm text-text-1">{a.start.toLocaleDateString()}</span>
        <a href="/activities/{a.id}" target="_blank" class="text-sm font-bold">
          &nbsp;{a.name}
        </a>
        <Badge color={prov === "strava" ? "orange" : "gray"} class="ml-2">
          {#if prov === "strava"}
            <img src="/strava.png" alt="" class="inline h-3.5 w-3.5" />
          {:else}
            icu
          {/if}
          {providerMeta[prov].name}
        </Badge>
      </div>
      <div class="flex gap-1 shrink-0">
        <Chip value={Math.round(a.distance / 1000) + "km"} label="" light />
      </div>
    </div>
  {/each}
</div>

<Section n="" title="Gear — prominent badge" />
<div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
  {#each mockGear as g}
    <div class="rounded-lg border border-border-subtle bg-surface-1 p-3">
      <div class="flex items-center justify-between">
        <span class="text-lg font-medium">{g.name}</span>
        <Badge color={g.provider === "strava" ? "orange" : "gray"}>
          {providerMeta[g.provider].name}
        </Badge>
      </div>
      <p class="text-sm text-text-1">{g.vendor} {g.model}</p>
    </div>
  {/each}
</div>

<Section n="④" title="Branding — partner headline" />
<div class="rounded-lg border border-border-subtle bg-surface-1 p-4">
  <p class="text-lg font-semibold">
    TendaBike +
    <img src="/strava.png" alt="Strava" class="inline h-5 w-5 align-middle" />
    Strava
  </p>
  <p class="text-sm text-text-1">
    Your maintenance tracker, powered by your Strava rides.
  </p>
</div>
