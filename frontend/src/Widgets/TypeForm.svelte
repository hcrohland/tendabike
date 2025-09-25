<script lang="ts">
  import { Input } from "flowbite-svelte";
  import { createEventDispatcher } from "svelte";
  import { Type, category } from "../lib/types";

  export let with_body = false;

  let hook: { type: Type; hook: number | null };
  const dispatch = createEventDispatcher();
</script>

<Input
  type="select"
  class="custom-select"
  required
  bind:value={hook}
  on:change={() => dispatch("change", hook)}
>
  <option hidden value> -- select one -- </option>
  {#if with_body}
    <option value={{ type: $category, hook: null }}> body </option>
  {/if}
  {#each $category.subtypes() as type}
    {#each type.hooks as hook}
      <option value={{ type, hook }}>
        {type.human_name(hook)}
      </option>
    {/each}
  {/each}
</Input>
