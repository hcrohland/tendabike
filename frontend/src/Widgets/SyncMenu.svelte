<script lang="ts">
  import { Dropdown, DropdownItem, NavLi } from "flowbite-svelte";
  import { handleError, myfetch, refresh, updateSummary } from "../lib/store";
  import { onDestroy } from "svelte";
  import { ChevronDownOutline } from "flowbite-svelte-icons";
  import Garmin from "../Activity/Garmin.svelte";

  let openGarmin = $state(false);

  let hook_timer = setTimeout(() => {});

  onDestroy(() => {
    clearInterval(hook_timer);
  });

  let hook_promise = $state(poll());

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

<DropdownItem class="cursor-pointer flex-end">
  {#await hook_promise}
    Syncing
  {:then}
    Sync
    <ChevronDownOutline class=" inline " />
  {:catch error}
    {handleError(error)}
  {/await}
</DropdownItem>
<Dropdown simple>
  <DropdownItem onclick={fullrefresh}>Refresh data</DropdownItem>
  <DropdownItem onclick={() => (openGarmin = true)}>With CSV File</DropdownItem>
</Dropdown>
<Garmin bind:open={openGarmin} />
