<script lang="ts">
  import { filterValues, services, by } from "../lib/store";
  import Usage from "../Usage.svelte";

  export let id: number;

  $: servs = filterValues($services, (s) => s.part_id == id).sort(by("time"));
</script>

{#if servs.length > 0}
  <div class="table-responsive">
    <table class="table">
      <thead>
        <tr>
          <th scope="col">Service</th>
          <th scope="col">Date</th>
          <Usage />
        </tr>
      </thead>
      <tbody>
        {#each servs as service (service.id)}
          <tr>
            <td>
              {service.name}
            </td>
            <td>{service.fmtTime()}</td>
            <Usage id={service.usage} ref={undefined} />
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}
