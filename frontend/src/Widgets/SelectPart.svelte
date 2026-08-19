<script lang="ts">
  import { parts } from "../lib/part";
  import { filterValues } from "../lib/mapable";
  import { category, Type } from "../lib/types";
  import { Select } from "flowbite-svelte";
  import { m } from "../../paraglide/messages";

  let {
    type,
    part = $bindable(),
    none,
  }: { type: Type; part: number | undefined; none?: boolean } = $props();

  let gears = $derived(
    filterValues($parts, (p) => type.main == p.what && !p.disposed_at),
  );
</script>

<Select
  required
  bind:value={part}
  placeholder={m.selectpart_placeholder({
    category: $category.localizedName(),
  })}
  classes={{ select: "rounded-l-none" }}
>
  {#if none}
    <option value={undefined}>{m.selectpart_none()}</option>
  {/if}
  {#each gears as gear}
    <option value={gear.id}>{gear.name}</option>
  {/each}
</Select>
