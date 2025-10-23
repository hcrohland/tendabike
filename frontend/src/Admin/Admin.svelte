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

  refresh();
</script>

{#await request}
  ...
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
    {#each list.sort((a, b) => a.user.id - b.user.id) as { user, parts, activities, events } (user.id)}
      <TableBodyRow>
        <TableBodyCell>{user.id}</TableBodyCell>
        <TableBodyCell>{user.firstname} {user.name}</TableBodyCell>
        <TableBodyCell>
          {user.is_admin ? "Admin" : "User"}
        </TableBodyCell>
        <TableBodyCell>{parts}</TableBodyCell>
        <TableBodyCell>{activities}</TableBodyCell>
        <TableBodyCell>{events}</TableBodyCell>
        <TableBodyCell>
          <ButtonGroup>
            <Button onclick={() => createSync.start(user)}>
              Add Sync Event
            </Button>
          </ButtonGroup>
        </TableBodyCell>
      </TableBodyRow>
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
