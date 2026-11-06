<!-- PROTOTYPE (throwaway) — Variant B: "Neutral Connect + deliberate picker".
     One neutral entry opens a picker; the connected app is deliberately
     provider-quiet (monochrome dots, neutral footnote). -->
<script lang="ts">
  import { Card, Button, Badge } from "flowbite-svelte";
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

<!-- ① CONNECT: one neutral entry (no brand) -->
<div class="rounded-lg border border-border-subtle bg-surface-1">
  <div class="flex items-center justify-between p-3">
    <div class="flex items-center gap-2">
      <img src="/favicon.png" alt="TendaBike" class="rounded-circle h-8" />
      <span class="font-semibold">Tend a Bike</span>
    </div>
    <button
      class="rounded-lg px-4 py-2 text-sm font-semibold text-white bg-blue-700 hover:bg-blue-800"
    >
      Connect an account
    </button>
  </div>
</div>

<Section n="①" title="Connect — one neutral button opens a picker" />
<div class="grid grid-cols-1 sm:grid-cols-2 gap-4 items-start">
  <!-- the neutral CTA -->
  <div
    class="rounded-xl border border-border-subtle bg-surface-1 p-6 text-center"
  >
    <p class="text-sm text-text-1 mb-4">
      One entry point — we'll ask which account to connect.
    </p>
    <Button color="blue">Get started</Button>
  </div>
  <!-- the picker (shown open, as it would appear in a modal) -->
  <div
    class="rounded-xl border-2 border-blue-500 bg-surface-1 p-4 shadow-md"
    aria-label="Provider picker (modal)"
  >
    <p class="text-sm font-semibold mb-3">Choose your training source</p>
    <div class="space-y-2">
      {#each Object.entries(providerMeta) as [key, meta]}
        <button
          class="w-full flex items-center gap-3 rounded-lg border border-border-subtle p-3 text-left hover:bg-surface-2"
        >
          <ProviderMark provider={key as "strava" | "icu"} size="md" />
          <span class="min-w-0">
            <span class="block font-medium">{meta.name}</span>
            <span class="block text-xs text-text-1">{meta.blurb}</span>
          </span>
        </button>
      {/each}
    </div>
  </div>
</div>

<Section n="②" title="Onboarding — chosen provider shown as a chip" />
<Card>
  <div class="text-center">
    <div class="flex justify-center mb-3">
      <Badge color="gray">
        Connected via <ProviderMark provider="icu" size="sm" />
        intervals.icu
      </Badge>
    </div>
    <h3 class="mb-3 text-lg font-semibold">Welcome to TendaBike!</h3>
    <p class="mb-4 text-left">
      Would you like to import your historic activities?
    </p>
    <div class="flex justify-center gap-3">
      <Button color="blue">Import Activities</Button>
      <Button color="alternative">Skip for Now</Button>
    </div>
  </div>
</Card>

<Section n="③" title="View-on links — a quiet monochrome dot" />
<div class="space-y-2">
  {#each mockActivities as a}
    {@const prov = providerOfActivityId(a.id)}
    <div
      class="rounded-lg border border-border-subtle bg-surface-1 p-3 flex items-center justify-between gap-2"
    >
      <div class="min-w-0">
        <span class="text-sm text-text-1">{a.start.toLocaleDateString()}</span>
        <a
          href="/activities/{a.id}"
          target="_blank"
          class="text-sm font-bold"
          title="View on {providerMeta[prov].name}"
        >
          &nbsp;{a.name}
          <span
            class="inline-block h-2 w-2 rounded-full bg-neutral-400 align-middle ml-1"
          ></span>
        </a>
      </div>
      <div class="flex gap-1 shrink-0">
        <Chip value={Math.round(a.distance / 1000) + "km"} label="" light />
        <Chip value={a.device_name} label="" light />
      </div>
    </div>
  {/each}
</div>

<Section n="" title="Gear — quiet dot + label" />
<div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
  {#each mockGear as g}
    <div class="rounded-lg border border-border-subtle bg-surface-1 p-3">
      <div class="text-lg font-medium">{g.name}</div>
      <p class="text-sm text-text-1">
        {g.vendor}
        {g.model}
        <span
          title="View on {providerMeta[g.provider].name}"
          class="ml-1 text-sm inline-flex items-center gap-1"
        >
          <span
            class="inline-block h-2 w-2 rounded-full bg-neutral-400 align-middle"
          ></span>
          {providerMeta[g.provider].name}
        </span>
      </p>
    </div>
  {/each}
</div>

<Section n="④" title="Branding — neutral footnote, both equal" />
<p class="text-text-1 text-sm">
  TendaBike works with
  <img
    src="/strava_grey.png"
    alt="Strava"
    class="inline h-4 w-4 align-middle"
  />
  Strava and
  <ProviderMark provider="icu" size="sm" /> intervals.icu.
</p>
