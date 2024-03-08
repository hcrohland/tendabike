<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import {
    Form,
    InputGroup,
    InputGroupText,
    Input,
  } from "@sveltestrap/sveltestrap";
  import { Type } from "../lib/types";
  import { types, category } from "../lib/types";
  import { filterValues } from "../lib/mapable";
  import { Part } from "../lib/part";

  export let gear: Part;
  export let with_body = false;

  let hook: { type: Type; hook: number | null };
  const dispatch = createEventDispatcher();

  let typeList = filterValues(
    types,
    (t) => t.main == $category.id && t.id != t.main,
  ).sort((a, b) => a.order - b.order);
</script>

<Form>
  <InputGroup class="col-md-12">
    <InputGroupText><slot>New</slot></InputGroupText>
    <!-- svelte-ignore a11y-no-onchange -->
    <Input
      type="select"
      class="custom-select"
      required
      bind:value={hook}
      on:change={() => dispatch("change", hook)}
    >
      <option hidden value> -- select one -- </option>
      {#if with_body}
        <option value={{ type: types[gear.what], hook: null }}> body </option>
      {/if}
      {#each typeList as type}
        {#each type.hooks as hook}
          <option value={{ type, hook }}>
            {type.human_name(hook)}
          </option>
        {/each}
      {/each}
    </Input>
    <InputGroupText>of {gear.name}</InputGroupText>
  </InputGroup>
</Form>
