<script lang="ts">
  import { filterValues } from "../lib/mapable.svelte";
  import { types, getCategory } from "../lib/types";
  import SpareType from "./SpareType.svelte";

  let attachee = $state(0);

  let spareTypes = $derived(
    filterValues(
      types,
      (t) => t.main == getCategory()!.id && t.id != getCategory()!.id,
    ),
  );

  function update(show: boolean) {
    show ? attachee++ : attachee--;
  }
</script>

<div class="flex flex-col gap-2">
  {#each spareTypes as type (type.id)}
    <SpareType {type} {attachee} {update} />
  {/each}
</div>
