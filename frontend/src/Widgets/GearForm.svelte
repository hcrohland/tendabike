<script lang="ts">
  import { Select } from "flowbite-svelte";
  import { category } from "../lib/types";
  import { allGear, parts } from "../lib/part";

  interface Props {
    gear: number | null;
  }

  let { gear = $bindable() }: Props = $props();

  let save = gear;
</script>

<Select bind:value={gear}>
  <option value={null}> any {$category.name.toLocaleLowerCase()} </option>
  {#each save ? [$parts[save]] : allGear($parts, $category) as part}
    <option value={part.id}>
      {part.name}
    </option>
  {/each}
</Select>
