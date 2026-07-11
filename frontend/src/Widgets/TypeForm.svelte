<script lang="ts">
  import { Select } from "flowbite-svelte";
  import { Type, category } from "../lib/types";
  import { m } from "../../paraglide/messages";

  interface Result {
    type: Type;
    hook: number | undefined;
  }
  interface Props {
    onChange: (t: Type, h: number | undefined) => void;
    with_body?: boolean;
    classes?: any;
  }

  let { onChange, with_body = false, ...rest }: Props = $props();

  let result: Result | undefined = $state();
</script>

<Select
  required
  bind:value={result}
  onchange={() => onChange(result!.type, result!.hook)}
  placeholder={m.typeform_choose_part()}
  {...rest}
>
  {#if with_body}
    <option value={{ type: $category, hook: null }}>{m.typeform_body()}</option>
  {/if}
  {#each $category.subtypes() as type}
    {#each type.hooks as hook}
      <option value={{ type, hook }}>
        {type.human_name(hook)}
      </option>
    {/each}
  {/each}
</Select>
