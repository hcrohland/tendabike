<script lang="ts">
  import { Button, ButtonGroup, Spinner, Table } from "@sveltestrap/sveltestrap";
  import { handleError, myfetch } from "../store";
  import type { User } from "../types";
  import Sync from "./Sync.svelte";
  import CreateSync from "./CreateSync.svelte";

  let promise, createSync;
  let request:
    | Promise<
        { user: User; parts: number; activities: number; events: number }[]
      >
    | any[] = [];

  function refresh() {
    request = myfetch("/user/all").catch(handleError);
  }

  function rescan() {
    promise = myfetch("/activ/rescan").catch(handleError);
    refresh();
  }

  async function disable(user) {
    await myfetch("/strava/disable/" + user.id, "POST").catch(handleError);
    refresh();
  }

  refresh();
</script>

{#await request}
  ...
{:then list}
  <Table>
    <tr>
      <th>Id</th>
      <th>Name</th>
      <th>Role</th>
      <th>Parts</th>
      <th>Activities</th>
      <th>Events</th>
      <th />
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
              <Button on:click={() => createSync(user)}>Add Sync Event</Button>
              <Sync {user} {refresh} />
              <Button on:click={() => disable(user)}>
                Disable user</Button
              >
            </ButtonGroup>
          {/if}
        </td>
      </tr>
    {/each}
  </Table>
  <ButtonGroup>
    <Button on:click={createSync()}>Add Sync Event for all</Button>
    <Button on:click={rescan}>
      {#await promise}
        <Spinner />
      {:then value}
        Rescan all activities
      {/await}
    </Button>
  </ButtonGroup>
  <Button on:click={refresh}>Refresh</Button>
{/await}
<CreateSync {refresh} bind:createSync />
