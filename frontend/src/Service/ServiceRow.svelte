<script lang="ts">
  import { DropdownItem, Tooltip } from "@sveltestrap/sveltestrap";
  import DeleteService from "./DeleteService.svelte";
  import UpdateService from "./UpdateService.svelte";
  import RedoService from "./RedoService.svelte";
  import UsageCells from "../Usage/Usage.svelte";
  import Menu from "../Widgets/Menu.svelte";
  import { Service, services } from "./service";
  import { Usage } from "../Usage/usage";

  export let depth: number;
  export let service: Service;
  export let days: number;
  export let usage: Usage;

  let updateService: (p: Service) => void;
  let redoService: (p: Service) => void;
  let deleteService: (p: Service) => void;
</script>

<tr>
  <td>
    <div>
      <span id={"name" + service.id}>
        {"┃ ".repeat(depth) + service.name}
        <slot />
      </span>
      {#if service.notes.length > 0}
        <Tooltip target={"name" + service.id}>
          {service.notes}
        </Tooltip>
      {/if}
    </div>
  </td>
  <td>{service.fmtTime($services)}</td>
  <td class="text-end">{days}</td>
  <UsageCells {usage} />
  <td>
    {#if !service.id?.includes("pred")}
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
