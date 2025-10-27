<script lang="ts">
  import { ButtonGroup, InputAddon } from "flowbite-svelte";
  import { handleError, myfetch } from "../lib/store";
  import type { User } from "../lib/types";
  import DateTime from "../Widgets/DateTime.svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import Switch from "../Widgets/Switch.svelte";
  import { by, filterValues } from "../lib/mapable";
  import { activities } from "../lib/activity";
  import Modal from "../Widgets/Modal.svelte";

  export let refresh: () => void;
  let user: User | undefined;
  let date = new Date();
  let open = false;
  let userParam: string;
  let checked = false;

  async function onaction() {
    await myfetch(
      "/strava/sync?time=" +
        (date.getTime() / 1000).toFixed(0) +
        "&migrate=" +
        checked +
        userParam,
    ).catch(handleError);
    open = false;
    refresh();
  }

  export const start = (u?: User) => {
    user = u;
    if (u) {
      userParam = "&user_id=" + u.id;
    } else {
      userParam = "";
    }
    open = true;
  };

  function prevdate(date: Date) {
    let prev = filterValues(
      $activities,
      (a) => a.user_id == user?.id && a.start < date,
    ).sort(by("start"))[0];
    return prev ? prev.start : date;
  }
</script>

<Modal bind:open {onaction}>
  {#snippet header()}
    Create sync Event for
    {#if user}
      {user.firstname} {user.name} ({user.id})
    {:else}
      all users
    {/if}
  {/snippet}
  <ButtonGroup>
    <InputAddon>Start</InputAddon>
    <DateTime bind:date prevdate={user ? prevdate : undefined} />
  </ButtonGroup>
  <Switch bind:checked>Migration</Switch>

  {#snippet footer()}
    <Buttons bind:open label="Sync" />
  {/snippet}
</Modal>
