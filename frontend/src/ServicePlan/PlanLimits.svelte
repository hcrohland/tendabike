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

  function handleChange(event: Event, key: keyof Limits) {
    const target = event.target as HTMLSelectElement;
    // @ts-ignore
    select[key] = Number(target.value);
  }

  // @ts-ignore
  let selected: ("days" | "hours" | "km" | "climb" | "descend" | "rides")[] =
    Object.entries(select)
      .filter(([k, v]) => Limits.keys.includes(k as any) && v != null)
      .map(([k, v]) => k);
</script>

{#each selected as key}
  <InputGroup>
    <Input
      type="number"
      placeholder={key}
      value={select[key]}
      on:input={(e) => handleChange(e, key)}
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
