<script lang="ts">
  import { Input } from "flowbite-svelte";
  import { category } from "../lib/types";
  import { allGear, parts } from "../lib/part";

  export let gear: number | null;

  let save = gear;
</script>

<Input type="select" class="custom-select" bind:value={gear}>
  <option value={null}> any {$category.name.toLocaleLowerCase()} </option>
  {#each save ? [$parts[save]] : allGear($parts, $category) as part}
    <option value={part.id}>
      {part.name}
    </option>
  {/each}
</Input>
