<script lang="ts">
  import { filterValues } from "../lib/mapable";
  import { types, category } from "../lib/types";
  import SpareType from "./SpareType.svelte";

  let attachee = $state(0);

  let spareTypes = $derived(
    filterValues(types, (t) => t.main == $category.id && t.id != $category.id),
  );

  function update(show: boolean) {
    show ? attachee++ : attachee--;
  }
</script>

<div class="flex flex-col gap-6">
  {#each spareTypes as type (type.id)}
    <SpareType {type} {attachee} {update} />
  {/each}
</div>
