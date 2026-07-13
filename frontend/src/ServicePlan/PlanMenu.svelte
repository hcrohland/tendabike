<script lang="ts">
  import { Dropdown, DropdownDivider, DropdownItem } from "flowbite-svelte";
  import { DotsVerticalOutline } from "flowbite-svelte-icons";
  import { actions } from "../Widgets/Actions.svelte";
  import { attachments } from "../lib/attachment";
  import type { ServicePlan } from "../lib/serviceplan";
  import { Part, parts } from "../lib/part";
  import { m } from "../../paraglide/messages";

  interface Props {
    plan: ServicePlan;
    name?: string | null;
  }

  let { plan, name = null }: Props = $props();
  let part = $derived(plan.getpart($parts, $attachments)) as Part;
</script>

<DotsVerticalOutline class="cursor-pointer float-inline-right inline" />
<Dropdown simple>
  {#if part}
    <DropdownItem onclick={() => $actions.newService(part, plan)}>
      {m.planmenu_new_service()}
    </DropdownItem>
    {#if plan.part != part.id}
      {@const att = part.attachments($attachments).at(0)}
      {#if att}
        <DropdownItem onclick={() => $actions.replacePart(att)}>
          {m.action_replace()}
        </DropdownItem>
      {/if}
    {/if}
  {/if}

  {#if !name && part}
    <DropdownDivider />
  {/if}

  {#if !name}
    <DropdownItem onclick={() => $actions.updatePlan(plan)}>
      {m.planmenu_change()}
    </DropdownItem>
    <DropdownItem onclick={() => $actions.deletePlan(plan)}>
      {m.planmenu_delete()}
    </DropdownItem>
  {/if}
</Dropdown>
