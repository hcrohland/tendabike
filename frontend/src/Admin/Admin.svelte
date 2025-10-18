<script lang="ts">
  import { Button, ButtonGroup, Spinner, Table } from "flowbite-svelte";
  import { handleError, myfetch, setSummary } from "../lib/store";
  import type { User } from "../lib/types";
  import Sync from "./Sync.svelte";
  import CreateSync from "./CreateSync.svelte";

  let promise: Promise<void>, createSync: any;
  let request:
    | Promise<
        { user: User; parts: number; activities: number; events: number }[]
      >
    | any[] = [];

  function refresh() {
    request = myfetch("/api/user/all").catch(handleError);
  }

  function rescan() {
    promise = myfetch("/api/activ/rescan")
      .catch(handleError)
      .then(refresh)
      .then(() => myfetch("/api/user/summary"))
      .then(setSummary);
  }

  async function disable(user: User) {
    await myfetch("/strava/disable/" + user.id, "POST").catch(handleError);
    refresh();
  }

  refresh();
</script>

{#await request}
  ...
{:then list: any[]}
  <Table>
    <tr>
      <th>Id</th>
      <th>Name</th>
      <th>Role</th>
      <th>Parts</th>
      <th>Activities</th>
      <th>Events</th>
      <th> </th>
    </tr>
    {#each list.sort((a, b) => a.user.id - b.user.id) as { user, parts, activities, events, disabled } (user.id)}
      <tr>
        <td> {user.id}</td>
        <td> {user.firstname} {user.name} </td>
        <td> {disabled ? "Disabled" : user.is_admin ? "Admin" : "User"}</td>
        <td> {parts}</td>
        <td> {activities}</td>
        <td> {events}</td>
        <td>
          {#if !disabled}
            <ButtonGroup>
              <Button onclick={() => createSync.start(user)}>
                Add Sync Event
              </Button>
              <Sync {user} {refresh} />
              <Button onclick={() => disable(user)}>Disable user</Button>
            </ButtonGroup>
          {/if}
        </td>
      </tr>
    {/each}
  </Table>
  <ButtonGroup>
    <Button onclick={createSync}>Add Sync Event for all</Button>
    <Button onclick={rescan}>
      {#await promise}
        <Spinner />
      {:then value}
        Rescan all activities
      {/await}
    </Button>
  </ButtonGroup>
  <Button onclick={refresh}>Refresh</Button>
{/await}
<CreateSync {refresh} bind:this={createSync} />
