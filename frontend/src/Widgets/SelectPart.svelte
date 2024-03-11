<script lang="ts">
  import { parts } from "../lib/part";
  import { filterValues } from "../lib/mapable";
  import { Type } from "../lib/types";
  import { Input } from "@sveltestrap/sveltestrap";
  import { createEventDispatcher } from "svelte";

  export let type: Type;
  export let part: number | undefined;
  export let none = false;

  const dispatch = createEventDispatcher();

  $: gears = filterValues($parts, (p) => type.main == p.what && !p.disposed_at);
</script>

<Input
  type="select"
  required
  bind:value={part}
  on:change={() => dispatch("change", part)}
>
  {#if none}
    <option value={undefined}> -- None -- </option>
  {:else}
    <option hidden value={null}> -- select one -- </option>
  {/if}
  {#each gears as gear}
    <option value={gear.id}>{gear.name}</option>
  {/each}
</Input>
