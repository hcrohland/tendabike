<script lang="ts">
  import PlanBadge from "../ServicePlan/PlanBadge.svelte";
  import { attachments } from "../lib/attachment";
  import { Part } from "../lib/part";
  import { plans, plans_for_part_and_subtypes } from "../lib/serviceplan";
  import GearCard from "./GearCard.svelte";

  export let part: Part;

  $: planlist = plans_for_part_and_subtypes($attachments, $plans, part);
</script>

<GearCard {part} summary>
  <PlanBadge {planlist} />
</GearCard>
