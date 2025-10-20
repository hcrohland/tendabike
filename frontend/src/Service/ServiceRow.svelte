<script lang="ts">
  import { TableBodyCell, TableBodyRow, Tooltip } from "flowbite-svelte";
  import UsageCells from "../Usage/Usage.svelte";
  import { Service } from "../lib/service";
  import { usages } from "../lib/usage";
  import { Part } from "../lib/part";
  import { Usage } from "../lib/usage";
  import { fmtRange, get_days } from "../lib/store";
  import ServiceMenu from "./ServiceMenu.svelte";

  interface Props {
    depth?: number;
    service?: Service | undefined;
    successor?: Service | null;
    part: Part;
    children?: import("svelte").Snippet;
  }

  let {
    depth = 0,
    service = undefined,
    successor = null,
    part,
    children,
  }: Props = $props();

  let usage = $derived(
    $usages[successor ? successor.usage : part.usage].sub(
      service ? $usages[service.usage] : new Usage(),
    ),
  );
  let days = $derived(
    get_days(
      service ? service.time : part.purchase,
      successor ? successor.time : new Date(),
    ),
  );
</script>

<TableBodyRow>
  {#if service}
    <TableBodyCell class="text-nowrap flex justify-between">
      <div>
        {@render children?.()}
        <span id={"name" + service.id}>
          {"┃ ".repeat(depth)}
          {service.name}
        </span>
        {#if service.notes.length > 0}
          <Tooltip>
            {service.notes}
          </Tooltip>
        {/if}
      </div>
      {#if service}
        <ServiceMenu {service} />
      {/if}
    </TableBodyCell>
    <TableBodyCell class="text-end">
      {fmtRange(service.time, successor?.time)}
    </TableBodyCell>
  {:else}
    <TableBodyCell>
      {"┃ ".repeat(depth)}┗━
    </TableBodyCell>
    <TableBodyCell class="text-end">
      {fmtRange(part.purchase, successor?.time)}
    </TableBodyCell>
  {/if}
  <TableBodyCell class="text-end">{days}</TableBodyCell>
  <UsageCells {usage} />
  <TableBodyCell></TableBodyCell>
</TableBodyRow>
