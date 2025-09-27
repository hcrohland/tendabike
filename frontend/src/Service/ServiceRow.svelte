<script lang="ts">
  import {
    DropdownItem,
    TableBodyCell,
    TableBodyRow,
    Tooltip,
  } from "flowbite-svelte";
  // import DeleteService from "./DeleteService.svelte";
  // import UpdateService from "./UpdateService.svelte";
  // import RedoService from "./RedoService.svelte";
  import UsageCells from "../Usage/Usage.svelte";
  // import Menu from "../Widgets/Menu.svelte";
  import { Service } from "../lib/service";
  import { usages } from "../lib/usage";
  import { Part } from "../lib/part";
  import { Usage } from "../lib/usage";
  import { fmtRange, get_days } from "../lib/store";
  import Menu from "../Widgets/Menu.svelte";

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

  let updateService: (p: Service | undefined) => void;
  let redoService: (p: Service | undefined) => void;
  let deleteService: (p: Service | undefined) => void;

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
    <TableBodyCell>
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
    </TableBodyCell>
    <TableBodyCell>{fmtRange(service.time, successor?.time)}</TableBodyCell>
  {:else}
    <TableBodyCell>
      {"┃ ".repeat(depth)}┗━
    </TableBodyCell>
    <TableBodyCell>{fmtRange(part.purchase, successor?.time)}</TableBodyCell>
  {/if}
  <TableBodyCell class="text-end">{days}</TableBodyCell>
  <UsageCells {usage} />
  <TableBodyCell>
    {#if service}
      <Menu>
        <DropdownItem onclick={() => alert("updateService(service)")}>
          Change Service
        </DropdownItem>
        <DropdownItem onclick={() => alert("redoService(service)")}>
          Repeat Service
        </DropdownItem>
        <DropdownItem onclick={() => alert("deleteService(service)")}>
          Delete Service
        </DropdownItem>
      </Menu>
    {/if}
  </TableBodyCell>
</TableBodyRow>
