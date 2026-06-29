<script lang="ts">
  import { Indicator } from "flowbite-svelte";
  import { ServicePlan, alerts_for_plans } from "../lib/serviceplan";
  import { parts } from "../lib/part";
  import { services } from "../lib/service";
  import { usages } from "../lib/usage";
  import { attachments } from "../lib/attachment";

  interface Props {
    planlist: ServicePlan[];
  }

  let { planlist }: Props = $props();

  let alerts = $derived(
    alerts_for_plans(planlist, $parts, $services, $usages, $attachments),
  );
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
