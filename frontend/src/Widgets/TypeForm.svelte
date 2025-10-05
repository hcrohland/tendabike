<script lang="ts">
  import { Select } from "flowbite-svelte";
  import { Type, category } from "../lib/types";

  interface Result {
    type: Type;
    hook: number | null;
  }
  interface Props {
    onChange: (t: Type, h: number | null) => void;
    with_body?: boolean;
  }

  let { onChange, with_body = false }: Props = $props();

  let result: Result | undefined = $state();
</script>

<Select
  required
  bind:value={result}
  onchange={() => onChange(result!.type, result!.hook)}
>
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
</Select>
