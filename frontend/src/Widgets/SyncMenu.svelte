<script lang="ts">
  import { Dropdown, DropdownItem, NavLi } from "flowbite-svelte";
  import { handleError, myfetch, refresh, updateSummary } from "../lib/store";
  import { onDestroy } from "svelte";
  import { ChevronDownOutline } from "flowbite-svelte-icons";
  // import Garmin from "../Activity/Garmin.svelte";

  let garmin: () => void = () => {};

  let hook_timer = setTimeout(() => {});

  onDestroy(() => {
    clearInterval(hook_timer);
  });

  let hook_promise = poll();

  async function poll() {
    clearInterval(hook_timer);
    let data;
    try {
      do {
        data = await myfetch("/strava/hooks");
        if (!data) break;
        updateSummary(data);
      } while (data["activities"].length > 0);
      hook_timer = setTimeout(() => {
        hook_promise = poll();
      }, 60000);
    } catch (e) {
      console.error(e);
      handleError(e as Error);
    }
  }

  function fullrefresh() {
    clearInterval(hook_timer);
    hook_promise = refresh().then(poll);
  }
</script>

<NavLi class="cursor-pointer flex-end">
  {#await hook_promise}
    Syncing
  {:then}
    &nbsp;&nbsp;&nbsp;&nbsp; Sync
    <ChevronDownOutline class=" inline " />
  {:catch error}
    {handleError(error)}
  {/await}
</NavLi>
<Dropdown simple>
  <DropdownItem onclick={fullrefresh}>Refresh data</DropdownItem>
  <DropdownItem onclick={garmin}>With CSV File</DropdownItem>
</Dropdown>
<!-- <Garmin bind:garmin /> -->
