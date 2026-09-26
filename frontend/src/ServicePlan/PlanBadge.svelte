<script lang="ts">
  import { Indicator } from "flowbite-svelte";
  import { plansForAssembly, alertCounts } from "../lib/serviceplan";
  import { Part } from "../lib/part";

  interface Props {
    part: Part;
  }

  let { part }: Props = $props();
  let planlist = $derived(plansForAssembly(part));
  let alerts = $derived(alertCounts(planlist));
</script>

<span class="relative -top-2 -right-1">
  {#if alerts.alert > 0}
    <Indicator color="red" class="text-xs text-gray-300 p-2">
      {alerts.alert + alerts.warn}
    </Indicator>
  {:else if alerts.warn > 0}
    <Indicator color="amber" class="text-xs text-gray-700 p-2">
      {alerts.warn}
    </Indicator>
  {/if}
</span>
