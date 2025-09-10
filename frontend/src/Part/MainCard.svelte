<script lang="ts">
  import { link } from "svelte-spa-router";
  import PlanBadge from "../ServicePlan/PlanBadge.svelte";
  import { attachments } from "../lib/attachment";
  import { Part } from "../lib/part";
  import { plans, plans_for_part_and_subtypes } from "../lib/serviceplan";
  import { category } from "../lib/types";
  import GearCard from "./GearCard.svelte";

  export let part: Part;

  $: planlist = plans_for_part_and_subtypes($attachments, $plans, part);
</script>

<GearCard {part}>
  <div class="float-end">
    <a
      href="/part/{part.id}"
      use:link
      class="badge text-bg-light text-decoration-none"
      title={"View " + $category.name.toLowerCase() + " details"}
    >
      <PlanBadge {planlist} />
      &Longrightarrow;
    </a>
  </div>
</GearCard>
