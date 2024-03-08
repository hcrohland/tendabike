<script lang="ts">
  let categories: number[];
  let promise = myfetch("/api/activ/categories").then(
    (data) => (categories = data),
  );
  import { myfetch, handleError } from "../lib/store";
  import {
    Dropdown,
    DropdownItem,
    DropdownMenu,
    DropdownToggle,
    Spinner,
  } from "@sveltestrap/sveltestrap";
  import { category, types } from "../lib/types";
</script>

<Dropdown direction="left">
  <DropdownToggle class="dropdown-item">Switch Sport</DropdownToggle>
  <DropdownMenu class="dropdown-submenu">
    {#await promise}
      <DropdownItem>
        <Spinner />
      </DropdownItem>
    {:then}
      {#each categories.sort() as cat}
        <DropdownItem on:click={() => category.set(types[cat])}>
          {types[cat].name}s
        </DropdownItem>
      {/each}
    {:catch error}
      {handleError(error)}
    {/await}
  </DropdownMenu>
</Dropdown>
