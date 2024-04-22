<script lang="ts">
  import { Badge } from "@sveltestrap/sveltestrap";
  import { ServicePlan, alerts_for_plans } from "../lib/serviceplan";
  import { parts } from "../lib/part";
  import { services } from "../lib/service";
  import { usages } from "../lib/usage";
  import { attachments } from "../lib/attachment";

  export let planlist: ServicePlan[];

  $: alerts = alerts_for_plans(
    planlist,
    $parts,
    $services,
    $usages,
    $attachments,
  );
</script>

{#if alerts.alert > 0}
  <Badge color="danger">{alerts.alert}</Badge>
{:else if alerts.warn > 0}
  <Badge color="warning">{alerts.warn}</Badge>
{/if}
