<script lang="ts">
  let categories: number[] = [];

  let promise: any;
  function ontoggle(e: any) {
    if (e.newState == "open") {
      promise = myfetch("/api/activ/categories").then(
        (data) => (categories = data),
      );
    }
  }

  import { myfetch, handleError } from "../lib/store";
  import { Dropdown, DropdownItem, Spinner } from "flowbite-svelte";
  import { category, types } from "../lib/types";
  import { ChevronDownOutline } from "flowbite-svelte-icons";
</script>

<DropdownItem class="cursor-pointer">
  Switch Sport
  <ChevronDownOutline class="inline" />
</DropdownItem>
<Dropdown simple {ontoggle}>
  {#await promise}
    <DropdownItem>
      <Spinner />
    </DropdownItem>
  {:then}
    {#each categories.sort() as cat}
      <DropdownItem onclick={() => category.set(types[cat])}>
        {types[cat].name}s
      </DropdownItem>
    {/each}
  {:catch error}
    {handleError(error)}
  {/await}
</Dropdown>
