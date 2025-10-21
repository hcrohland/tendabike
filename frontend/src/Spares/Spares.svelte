<script lang="ts">
  import { filterValues } from "../lib/mapable";
  import { types, category } from "../lib/types";
  import Usage from "../Usage/Usage.svelte";
  import SpareType from "./SpareType.svelte";
  import { Table, TableBody, TableHead, TableHeadCell } from "flowbite-svelte";

  let attachee = $state(0);

  let spareTypes = $derived(
    filterValues(types, (t) => t.main == $category.id && t.id != $category.id),
  );

  function update(show: boolean) {
    show ? attachee++ : attachee--;
  }
</script>

<div class="table-responsive">
  <Table hoverable striped>
    <TableHead>
      <TableHeadCell colspan={3} scope="col">&NonBreakingSpace;</TableHeadCell>
      <Usage header />
      {#if attachee > 0}
        <TableHeadCell colspan={2}>Attached to</TableHeadCell>
      {/if}
    </TableHead>
    <TableBody>
      {#each spareTypes as type (type.id)}
        <SpareType {type} {attachee} {update} />
      {/each}
    </TableBody>
  </Table>
</div>
