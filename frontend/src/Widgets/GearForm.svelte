<script lang="ts">
  import { Select } from "flowbite-svelte";
  import { category } from "../lib/types";
  import { allGear, parts } from "../lib/part";
  import { m } from "../../paraglide/messages";

  interface Props {
    gear: number | null;
  }

  let { gear = $bindable() }: Props = $props();

  let save = gear;
</script>

<Select bind:value={gear} classes={{ select: "rounded-l-none h-full" }}>
  <option value={null}>
    {m.gearform_any({
      category: $category.localizedName().toLocaleLowerCase(),
    })}
  </option>
  {#each save ? [$parts[save]] : allGear($parts, $category) as part}
    <option value={part.id}>
      {part.name}
    </option>
  {/each}
</Select>
