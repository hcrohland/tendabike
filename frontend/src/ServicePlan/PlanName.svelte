<script lang="ts">
  import { plans, ServicePlan } from "../lib/serviceplan";
  import { category, types } from "../lib/types";
  import { link } from "svelte-spa-router";
  import { attachments, part_at_hook } from "../lib/attachment";
  import { parts } from "../lib/part";

  export let plan: ServicePlan | undefined;
  export let full = false;
</script>

{#if plan}
  {plan.name}
  {#if plan.hook}
    {#if plan.part}
      <a
        href="/part/{part_at_hook(
          plan.part,
          plan.what,
          plan.hook,
          $attachments,
        )}"
        use:link
        class="text-reset"
      >
        {types[plan.what].human_name(plan.hook)}
      </a>
    {:else}
      {types[plan.what].human_name(plan.hook)}
    {/if}
    {#if plan.no_template($plans)}
      for {@html $parts[plan.part].partLink()}
    {:else}
      at any {$category.name.toLocaleLowerCase()}
    {/if}
  {:else if full}
    for {@html $parts[plan.part].partLink()}
  {/if}
{/if}
