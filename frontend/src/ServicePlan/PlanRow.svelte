<script lang="ts">
  import { TableBodyCell, TableBodyRow, Tooltip } from "flowbite-svelte";
  import ServiceRow from "../Service/ServiceRow.svelte";
  import { attachments } from "../lib/attachment";
  import { Part, parts } from "../lib/part";
  import { services } from "../lib/service";
  import { Limits, ServicePlan } from "../lib/serviceplan";
  import { usages } from "../lib/usage";
  import PlanName from "./PlanName.svelte";
  import ShowMore from "../Widgets/ShowMore.svelte";
  import PlanMenu from "./PlanMenu.svelte";
  import { fmtNumber } from "../lib/store";

  interface Props {
    plan: ServicePlan;
    name?: string | null;
  }

  let { plan, name = null }: Props = $props();

  let show_more = $state(false);

  let part = $derived(plan.getpart($parts, $attachments)) as Part;
  let serviceList = $derived(plan.services(part, $services));
  let due = $derived(plan.due(part, serviceList.at(0), $usages));
  let title = "service history";

  function get_class(plan: number, due: number) {
    if (due < 0) return "rounded p-1 bg-red-600 text-white";
    if (due < plan * 0.05) return "rounded p-1 text-gray-900 bg-yellow-200";
    return "";
  }
</script>

{#snippet cell(key: keyof Limits)}
  {@const p = plan[key] as number}
  {@const d = due[key] as number}
  <td class="text-end">
    {#if p != null && d != null}
      <span class={get_class(p, d)}>
        {fmtNumber(d)}
      </span>
      <Tooltip>
        {fmtNumber(p - d)} / {fmtNumber(p)}
      </Tooltip>
    {:else}
      -
    {/if}
  </td>
{/snippet}

<TableBodyRow>
  <TableBodyCell colspan={2}>
    <div class="text-nowrap flex justify-between">
      <div>
        {#if name}
          ┃
          <ShowMore bind:show_more {title} />
          {@html name}
        {:else}
          {#if part}
            <ShowMore bind:show_more {title} />
          {/if}
          <PlanName {plan} /> in
        {/if}
      </div>
      <PlanMenu {plan} {name} />
    </div>
  </TableBodyCell>
  {#if part}
    {#each Limits.keys as key}
      {@render cell(key as any)}
    {/each}
  {/if}
</TableBodyRow>
{#if part && show_more}
  {#each serviceList as service, i (service.id)}
    {@const successor = i > 0 ? serviceList[i - 1] : null}
    <ServiceRow depth={name ? 2 : 1} {part} {service} {successor} />
  {/each}
  <ServiceRow depth={name ? 1 : 0} {part} successor={serviceList.at(-1)} />
{/if}
