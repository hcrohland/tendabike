<script lang="ts">
  import {
    Button,
    Dropdown,
    DropdownItem,
    DropdownMenu,
    DropdownToggle,
    Input,
    InputGroup,
    InputGroupText,
  } from "@sveltestrap/sveltestrap";
  import { Limits } from "./serviceplan";

  export let select: Limits;

  let selected: (
    | "days"
    | "time"
    | "distance"
    | "climb"
    | "descend"
    | "count"
  )[] = [];
</script>

{#each selected as key}
  <InputGroup>
    <Input
      type="number"
      placeholder={key}
      bind:value={select[key]}
      on:change={() => {
        let s = select[key];
        select[key] = Number(s);
      }}
    />
    <InputGroupText>{key}</InputGroupText>
    <Button
      color="light"
      on:click={() => {
        select[key] = null;
        selected = selected.filter((k) => k != key);
      }}>⊗</Button
    >
  </InputGroup>
{/each}
<Dropdown direction="left" class="float-end">
  <DropdownToggle color="light">add limit</DropdownToggle>
  <DropdownMenu>
    {#each Limits.keys.filter((k) => !selected.includes(k)) as key}
      <DropdownItem
        on:click={() => {
          selected.push(key);
          selected = selected;
        }}
      >
        {key}
      </DropdownItem>
    {/each}
  </DropdownMenu>
</Dropdown>
