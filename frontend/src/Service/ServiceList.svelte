<script lang="ts">
  import { Button, DropdownItem, Tooltip } from "@sveltestrap/sveltestrap";
  import { services, usages, filterValues, by } from "../lib/store";
  import DeleteService from "./DeleteService.svelte";
  import UpdateService from "./UpdateService.svelte";
  import RedoService from "./RedoService.svelte";
  import NewService from "./NewService.svelte";
  import Usage from "../Usage.svelte";
  import Menu from "../Widgets/Menu.svelte";
  import { Service } from "./service";
  import { Part } from "../lib/types";

  export let part: Part;

  let newService: (p: Part) => void;
  let updateService: (p: Service) => void;
  let redoService: (p: Service) => void;
  let deleteService: (p: Service) => void;

  $: servs = filterValues($services, (s) => s.part_id == part.id).sort(
    by("time"),
  );
</script>

{#if servs.length > 0}
  <div class="table-responsive">
    <table class="table">
      <thead>
        <tr>
          <th scope="col">
            Service Log&nbsp;&nbsp;
            <Button size="sm" color="light" on:click={() => newService(part)}>
              add
            </Button>
          </th>
          <th scope="col">Date</th>
          <Usage />
          <th />
        </tr>
      </thead>
      <tbody>
        {#each servs as service (service.id)}
          <tr>
            <td>
              <div>
                <span id={"name" + service.id}>
                  {service.name}
                </span>
                {#if service.notes.length > 0}
                  <Tooltip target={"name" + service.id}>
                    {service.notes}
                  </Tooltip>
                {/if}
              </div>
            </td>
            <td>{service.fmtTime($services)}</td>
            <Usage usage={service.get_use(part, $usages, $services)} />
            <td>
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
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}
<NewService bind:newService />
<UpdateService bind:updateService />
<DeleteService bind:deleteService />
<RedoService bind:redoService />
