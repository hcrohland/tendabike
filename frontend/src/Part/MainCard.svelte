<script lang="ts">
  import GearCard from "./GearCard.svelte";
  import { Part } from "../lib/part";
  import { category } from "../lib/types";
  import { link } from "svelte-spa-router";
  import { plans, plans_for_part_and_attachees } from "../lib/serviceplan";
  import { attachments } from "../lib/attachment";
  import PlanBadge from "../ServicePlan/PlanBadge.svelte";

  export let part: Part;
  export let display: boolean;

  $: planlist = plans_for_part_and_attachees($attachments, $plans, part.id);
</script>

<GearCard {part} {display}>
  <div class="float-end">
    <a
      href="/part/{part.id}"
      use:link
      class="badge badge-secondary text-decoration-none"
      title={"View " + $category.name.toLowerCase() + " details"}
    >
      <PlanBadge {planlist} />
      &Longrightarrow;
    </a>
  </div>
</GearCard>
