<script lang="ts">
  import PlanBadge from "../ServicePlan/PlanBadge.svelte";
  import { Part } from "../lib/part";
  import GearCard from "./GearCard.svelte";

  export let part: Part;
</script>

<GearCard {part} summary gridclass="grid-cols-3">
  <PlanBadge {part} />
</GearCard>
