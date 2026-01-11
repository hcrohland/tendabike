<script lang="ts">
  import {
    Button,
    ButtonGroup,
    Spinner,
    Table,
    TableBodyCell,
    TableBodyRow,
    TableHeadCell,
  } from "flowbite-svelte";
  import { handleError, myfetch, setSummary } from "../lib/store";
  import type { User } from "../lib/types";
  import Sync from "./Sync.svelte";
  import CreateSync from "./CreateSync.svelte";
  import DeleteUser from "./DeleteUser.svelte";

  let promise: Promise<void>, createSync: any;
  let request:
    | Promise<
        { user: User; parts: number; activities: number; events: number }[]
      >
    | any[] = [];

  function refresh() {
    request = myfetch("/api/user/all").catch(handleError);
  }

  let deleteuser = { start: (_: User) => {} };

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
  <Spinner size="16" class="justify-center" />
{:then list: any[]}
  <Table>
    <TableBodyRow>
      <TableHeadCell>Id</TableHeadCell>
      <TableHeadCell>Name</TableHeadCell>
      <TableHeadCell>Role</TableHeadCell>
      <TableHeadCell>Parts</TableHeadCell>
      <TableHeadCell>Activities</TableHeadCell>
      <TableHeadCell>Events</TableHeadCell>
      <TableHeadCell></TableHeadCell>
    </TableBodyRow>
    {#each list.sort((a, b) => a.user.id - b.user.id) as { user, parts, activities, events, disabled } (user.id)}
      <TableBodyRow>
        <TableBodyCell>{user.id}</TableBodyCell>
        <TableBodyCell>{user.firstname} {user.name}</TableBodyCell>
        <TableBodyCell>
          {disabled
            ? "Disabled"
            : user.is_admin
              ? "Admin"
              : "User"}</TableBodyCell
        >
        <TableBodyCell>{parts}</TableBodyCell>
        <TableBodyCell>{activities}</TableBodyCell>
        <TableBodyCell>{events}</TableBodyCell>
        <TableBodyCell>
          <ButtonGroup>
            {#if !disabled}
              <Button onclick={() => createSync.start(user)}>
                Add Sync Event
              </Button>
              <Sync {user} {refresh} />
              <Button onclick={() => disable(user)}>Disable user</Button>
            {/if}
            <Button
              onclick={() => {
                deleteuser?.start(user);
              }}>Delete User</Button
            >
          </ButtonGroup>
        </TableBodyCell>
      </TableBodyRow>
    {/each}
  </Table>
  <ButtonGroup class="p-6">
    <Button onclick={() => createSync.start()}>Add Sync Event for all</Button>
    <Button onclick={rescan}>
      {#await promise}
        <Spinner />
      {:then}
        Rescan all activities
      {/await}
    </Button>
  </ButtonGroup>
  <Button onclick={refresh}>Refresh</Button>
{/await}
<CreateSync {refresh} bind:this={createSync} />
<DeleteUser bind:this={deleteuser} {refresh} />
