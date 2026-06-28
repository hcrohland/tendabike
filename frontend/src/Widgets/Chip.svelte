<script lang="ts">
  import { link } from "svelte-spa-router";
  import ServiceBadge from "./ServiceBadge.svelte";

  export let label: string;
  export let value: string;
  export let href: string | undefined = undefined;
  export let light = false;
  export let service: { due: number; plan: number } | undefined = undefined;

  let background = !light
    ? "bg-gray-50 dark:bg-gray-700"
    : "bg-gray-250 dark:bg-gray-800";
</script>

<div class={"relative rounded-lg shrink-0 " + background}>
  <ServiceBadge {service} pos="absolute -top-0 -right-0"></ServiceBadge>
  {#if href}
    <a
      {href}
      use:link
      class="flex items-center gap-1 px-3 py-2 text-reset no-underline"
    >
      <span class="font-semibold text-sm">{value}</span>
      <span
        class="font-normal text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400"
        >{label}</span
      >
    </a>
  {:else}
    <div class={"flex items-center gap-1 px-3 py-2 "}>
      <span class="font-semibold text-sm">{value}</span>
      <span
        class="font-normal text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400"
        >{label}</span
      >
    </div>
  {/if}
</div>
