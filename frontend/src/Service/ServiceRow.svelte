<script lang="ts">
  import { DropdownItem, Tooltip } from "@sveltestrap/sveltestrap";
  import DeleteService from "./DeleteService.svelte";
  import UpdateService from "./UpdateService.svelte";
  import RedoService from "./RedoService.svelte";
  import UsageCells from "../Usage/Usage.svelte";
  import Menu from "../Widgets/Menu.svelte";
  import { Service } from "./service";
  import { usages } from "../Usage/usage";
  import { Part } from "../lib/part";
  import { Usage } from "../Usage/usage";
  import { fmtRange, get_days } from "../lib/store";

  export let depth: number = 0;
  export let service: Service | undefined = undefined;
  export let successor: Service | null = null;
  export let part: Part;

  let updateService: (p: Service | undefined) => void;
  let redoService: (p: Service | undefined) => void;
  let deleteService: (p: Service | undefined) => void;

  $: usage = $usages[successor ? successor.usage : part.usage].sub(
    service ? $usages[service.usage] : new Usage(),
  );
  $: days = get_days(
    service ? service.time : part.purchase,
    successor ? successor.time : new Date(),
  );
</script>

<tr>
  {#if service}
    <td>
      <div>
        <span id={"name" + service.id}>
          {"┃ ".repeat(depth)}
          {service.name}
        </span>
        {#if service.notes.length > 0}
          <Tooltip target={"name" + service.id}>
            {service.notes}
          </Tooltip>
        {/if}
        <slot />
      </div>
    </td>
    <td>{fmtRange(service.time, successor?.time)}</td>
  {:else}
    <td>
      {"┃ ".repeat(depth)}┗━
    </td>
    <td>{fmtRange(part.purchase, successor?.time)}</td>
  {/if}
  <td class="text-end">{days}</td>
  <UsageCells {usage} />
  <td>
    {#if service}
      <Menu>
        <DropdownItem on:click={() => updateService(service)}>
          Change Service
        </DropdownItem>
        <DropdownItem on:click={() => redoService(service)}>
          Repeat Service
        </DropdownItem>
        <DropdownItem on:click={() => deleteService(service)}>
          Delete Service
        </DropdownItem>
      </Menu>
    {/if}
  </td>
</tr>
<UpdateService bind:updateService />
<DeleteService bind:deleteService />
<RedoService bind:redoService />
