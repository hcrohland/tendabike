<script lang="ts">
  import {
    Dropdown,
    DropdownItem,
    DropdownMenu,
    DropdownToggle,
  } from "@sveltestrap/sveltestrap";
  import { handleError, myfetch, refresh, updateSummary } from "../lib/store";
  import { onDestroy } from "svelte";
  import Garmin from "../Activity/Garmin.svelte";

  let garmin: () => void;

  let syncOpen: boolean;

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

<Dropdown nav isOpen={syncOpen} toggle={() => (syncOpen = !syncOpen)}>
  <DropdownToggle color="light" caret>
    {#await hook_promise}
      Syncing
    {:then}
      &nbsp;&nbsp;&nbsp;&nbsp; Sync
    {:catch error}
      {handleError(error)}
    {/await}
  </DropdownToggle>
  <DropdownMenu right>
    <DropdownItem on:click={fullrefresh}>Refresh data</DropdownItem>
    <DropdownItem on:click={garmin}>With CSV File</DropdownItem>
  </DropdownMenu>
</Dropdown>
<Garmin bind:garmin />
