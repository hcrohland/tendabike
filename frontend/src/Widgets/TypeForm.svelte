<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import {
    Form,
    InputGroup,
    InputGroupText,
    Input,
  } from "@sveltestrap/sveltestrap";
  import { Type } from "../lib/types";
  import { types, category } from "../lib/store";
  import { filterValues } from "../lib/mapable";
  import { Part } from "../Part/part";

  export let gear: Part;

  let hook: { type: Type; hook: number };
  const dispatch = createEventDispatcher();

  let typeList = filterValues(
    types,
    (t) => t.main == $category.id && t.id != t.main,
  ).sort((a, b) => a.order - b.order);

  $: if (hook) dispatch("change", { ...hook });
</script>

<Form>
  <InputGroup class="col-md-12">
    <InputGroupText><slot>New</slot></InputGroupText>
    <!-- svelte-ignore a11y-no-onchange -->
    <Input type="select" class="custom-select" required bind:value={hook}>
      <option hidden value> -- select one -- </option>
      {#each typeList as type}
        {#each type.hooks as hook}
          <option value={{ type, hook }}>
            {#if type.hooks.length > 1}
              {types[hook].prefix}
            {/if}
            {type.name}
          </option>
        {/each}
      {/each}
    </Input>
    <InputGroupText>of {gear.name}</InputGroupText>
  </InputGroup>
</Form>
