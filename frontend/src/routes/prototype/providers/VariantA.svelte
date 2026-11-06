<!-- PROTOTYPE (throwaway) — Variant A: "Inline choice".
     The provider is a peer: a second brand button appears wherever Strava
     does. Small per-provider icons on cards; both logos in the footer. -->
<script lang="ts">
  import { Card, Button } from "flowbite-svelte";
  import Chip from "../../../Widgets/Chip.svelte";
  import ProviderMark from "./ProviderMark.svelte";
  import Section from "./Section.svelte";
  import {
    mockActivities,
    mockGear,
    providerOfActivityId,
    providerMeta,
  } from "./mock";
</script>

<!-- ① CONNECT: two peer brand buttons in the header -->
<div class="rounded-lg border border-border-subtle bg-surface-1">
  <div class="flex items-center justify-between p-3">
    <div class="flex items-center gap-2">
      <img src="/favicon.png" alt="TendaBike" class="rounded-circle h-8" />
      <span class="font-semibold">Tend a Bike</span>
    </div>
    <div class="flex items-center gap-2">
      <a
        href="#/strava/login"
        class="inline-flex items-center gap-2 rounded-lg px-3 py-2 text-sm font-semibold text-white"
        style="background:#fc4c02"
      >
        <img src="/strava.png" alt="" class="h-5 w-5" /> Connect with Strava
      </a>
      <a
        href="#/icu/login"
        class="inline-flex items-center gap-2 rounded-lg px-3 py-2 text-sm font-semibold text-white bg-neutral-800 dark:bg-neutral-200 dark:text-neutral-900"
      >
        <ProviderMark provider="icu" size="sm" /> Connect with intervals.icu
      </a>
    </div>
  </div>
</div>

<Section n="①" title="Connect — a second peer button" />
<div class="flex flex-col sm:flex-row gap-3">
  <a
    href="#/strava/login"
    class="flex-1 flex items-center justify-center gap-3 rounded-xl border-2 p-4 font-semibold"
    style="border-color:#fc4c02; color:#fc4c02"
  >
    <img src="/strava.png" alt="" class="h-8 w-8" />
    <span>
      Connect with Strava
      <span class="block text-xs font-normal opacity-70">
        {providerMeta.strava.blurb}
      </span>
    </span>
  </a>
  <a
    href="#/icu/login"
    class="flex-1 flex items-center justify-center gap-3 rounded-xl border-2 p-4 font-semibold border-neutral-800 dark:border-neutral-200 text-neutral-900 dark:text-neutral-100"
  >
    <ProviderMark provider="icu" size="lg" />
    <span>
      Connect with intervals.icu
      <span class="block text-xs font-normal opacity-70">
        {providerMeta.icu.blurb}
      </span>
    </span>
  </a>
</div>

<Section n="②" title="Onboarding — provider named in the dialog" />
<Card>
  <div class="text-center">
    <h3 class="mb-3 text-lg font-semibold">Welcome to TendaBike!</h3>
    <p class="mb-4 text-left">
      Would you like to import your historic activities from
      <strong> Strava</strong>?
      <ProviderMark provider="strava" size="sm" />
    </p>
    <div class="flex justify-center gap-3">
      <Button color="blue">Import Activities</Button>
      <Button color="alternative">Skip for Now</Button>
    </div>
  </div>
</Card>

<Section n="③" title="View-on links — a small per-provider icon" />
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
          <ProviderMark provider={prov} size="sm" />
        </a>
      </div>
      <div class="flex gap-1 shrink-0">
        <Chip value={Math.round(a.distance / 1000) + "km"} label="" light />
        <Chip value={a.device_name} label="" light />
      </div>
    </div>
  {/each}
</div>

<Section n="" title="Gear — per-provider icon + label" />
<div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
  {#each mockGear as g}
    <div class="rounded-lg border border-border-subtle bg-surface-1 p-3">
      <div class="text-lg font-medium">{g.name}</div>
      <p class="text-sm text-text-1">
        {g.vendor}
        {g.model}
        <span class="inline-flex items-center gap-1 ml-1 text-sm">
          <ProviderMark provider={g.provider} size="sm" />
          View on {providerMeta[g.provider].name}
        </span>
      </p>
    </div>
  {/each}
</div>

<Section n="④" title="Branding — both logos, side by side" />
<div class="flex items-center gap-3 text-text-1 text-sm">
  Powered by
  <img src="/strava.png" alt="Strava" class="h-5 w-5" />
  <ProviderMark provider="icu" size="md" />
</div>
