<script lang="ts">
  import { ServicePlan, isTemplate } from "../lib/serviceplan";
  import { getCategory, types } from "../lib/types";
  import { link } from "svelte-spa-router";
  import { attachments, part_at_hook } from "../lib/attachment";
  import { parts } from "../lib/part";
  import { m } from "../../paraglide/messages";

  interface Props {
    plan: ServicePlan | undefined;
  }

  let { plan }: Props = $props();

  let partlink = $derived(plan?.part ? parts[plan.part].partLink() : "");
</script>

{#if plan}
  {plan.name}
  {m.planname_for()}
  {#if plan.hook}
    {#if plan.part}
      <a
        href="/part/{part_at_hook(
          plan.part,
          plan.what,
          plan.hook,
          attachments,
        )}"
        use:link
      >
        {types[plan.what].human_name(plan.hook)}
      </a>
    {:else}
      {types[plan.what].human_name(plan.hook)}
    {/if}
    {#if isTemplate(plan)}
      {m.attachform_of()} {getCategory()!.localizedAnyDative()}
    {:else}
      {m.attachform_of()} {@html partlink}
    {/if}
  {:else}
    {@html partlink}
  {/if}
{/if}
