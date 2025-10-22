<script lang="ts">
  import { Dropdown, DropdownDivider, DropdownItem } from "flowbite-svelte";
  import { ChevronDownOutline } from "flowbite-svelte-icons";
  import { actions } from "../Widgets/Actions.svelte";
  import { attachments } from "../lib/attachment";
  import type { ServicePlan } from "../lib/serviceplan";
  import { Part, parts } from "../lib/part";

  interface Props {
    plan: ServicePlan;
    name?: string | null;
  }

  let { plan, name = null }: Props = $props();
  let part = $derived(plan.getpart($parts, $attachments)) as Part;
</script>

<ChevronDownOutline class="cursor-pointer float-inline-right inline" />
<Dropdown simple>
  {#if part}
    {@const plans = plan.id ? [plan.id] : []}
    <DropdownItem onclick={() => $actions.newService(part, plan)}>
      New Service for plan
    </DropdownItem>
    {#if plan.part != part.id}
      {@const att = part.attachments($attachments).at(0)}
      {#if att}
        <DropdownItem onclick={() => $actions.replacePart(att)}>
          Replace Part
        </DropdownItem>
      {/if}
    {/if}
  {/if}

  {#if !name && part}
    <DropdownDivider />
  {/if}

  {#if !name}
    <DropdownItem onclick={() => $actions.updatePlan(plan)}>
      Change ServicePlan
    </DropdownItem>
    <DropdownItem onclick={() => $actions.deletePlan(plan)}>
      Delete ServicePlan
    </DropdownItem>
  {/if}
</Dropdown>
