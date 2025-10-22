<script lang="ts">
  import { plans, ServicePlan } from "../lib/serviceplan";
  import { category, types } from "../lib/types";
  import { link } from "svelte-spa-router";
  import { attachments, part_at_hook } from "../lib/attachment";
  import { parts } from "../lib/part";

  interface Props {
    plan: ServicePlan | undefined;
  }

  let { plan }: Props = $props();

  let partlink = $derived(plan?.part ? $parts[plan.part].partLink() : "");
</script>

{#if plan}
  {plan.name} for
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
      >
        {types[plan.what].human_name(plan.hook)}
      </a>
    {:else}
      {types[plan.what].human_name(plan.hook)}
    {/if}
    {#if plan.no_template($plans)}
      of {@html partlink}
    {:else}
      of any {$category.name.toLocaleLowerCase()}
    {/if}
  {:else}
    {@html partlink}
  {/if}
{/if}
