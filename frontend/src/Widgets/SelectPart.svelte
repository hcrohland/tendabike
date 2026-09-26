<script lang="ts">
  import { parts } from "../lib/part";
  import { stateValues } from "../lib/mapable.svelte";
  import { getCategory, Type } from "../lib/types";
  import { Select } from "flowbite-svelte";
  import { m } from "../../paraglide/messages";

  let {
    type,
    part = $bindable(),
    none,
  }: { type: Type; part: number | undefined; none?: boolean } = $props();

  let gears = $derived(
    stateValues(parts).filter((p) => type.main == p.what && !p.disposed_at),
  );
</script>

<Select
  required
  bind:value={part}
  placeholder={m.selectpart_placeholder({
    category: getCategory()!.localizedName(),
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
