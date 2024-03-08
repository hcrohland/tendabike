<script lang="ts">
  import type { ServicePlan } from "../lib/serviceplan";
  import { types } from "../lib/store";
  import { link } from "svelte-spa-router";
  import { attachments, part_at_hook } from "../lib/attachment";

  export let plan: ServicePlan;
</script>

{#if plan.hook}
  <slot />
  <a
    href="/part/{part_at_hook(plan.part, plan.what, plan.hook, $attachments)}"
    use:link
    class="text-reset"
  >
    {types[plan.what].human_name(plan.hook)}
  </a>
{/if}
