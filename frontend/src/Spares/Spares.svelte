<script lang="ts">
  import { filterValues, types, category } from "../lib/store";
  import Usage from "../Usage.svelte";
  import SpareType from "./SpareType.svelte";
  import { Table } from "@sveltestrap/sveltestrap";

  let attachee = 0;

  $: spareTypes = filterValues(
    types,
    (t) => t.main == $category.id && t.id != $category.id,
  );

  function update(show: boolean) {
    show ? attachee++ : attachee--;
  }
</script>

<div class="table-responsive">
  <Table responsive hover>
    <thead>
      <tr>
        <th scope="col">Part</th>
        <th scope="col">Name</th>
        <Usage header />
        {#if attachee > 0}
          <th colspan="2"> Attached to </th>
        {/if}
      </tr>
    </thead>
    <tbody>
      {#each spareTypes as type (type.id)}
        <SpareType {type} {attachee} {update} />
      {/each}
    </tbody>
  </Table>
</div>
