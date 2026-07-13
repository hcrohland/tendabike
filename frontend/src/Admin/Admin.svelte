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
  import { handleError, myfetch } from "../lib/store";
  import { type User, setSummary } from "../lib/user";
  import Sync from "./Sync.svelte";
  import CreateSync from "./CreateSync.svelte";
  import DeleteUser from "./DeleteUser.svelte";
  import * as m from "../../paraglide/messages";

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
      <TableHeadCell>{m.admin_col_id()}</TableHeadCell>
      <TableHeadCell>{m.partform_name()}</TableHeadCell>
      <TableHeadCell>{m.admin_col_role()}</TableHeadCell>
      <TableHeadCell>{m.nav_parts()}</TableHeadCell>
      <TableHeadCell>{m.nav_activities()}</TableHeadCell>
      <TableHeadCell>{m.admin_col_events()}</TableHeadCell>
      <TableHeadCell></TableHeadCell>
    </TableBodyRow>
    {#each list.sort((a, b) => a.user.id - b.user.id) as { user, parts, activities, events, disabled } (user.id)}
      <TableBodyRow>
        <TableBodyCell>{user.id}</TableBodyCell>
        <TableBodyCell>{user.firstname} {user.name}</TableBodyCell>
        <TableBodyCell>
          {disabled
            ? m.admin_role_disabled()
            : user.is_admin
              ? m.header_admin()
              : m.admin_role_user()}</TableBodyCell
        >
        <TableBodyCell>{parts}</TableBodyCell>
        <TableBodyCell>{activities}</TableBodyCell>
        <TableBodyCell>{events}</TableBodyCell>
        <TableBodyCell>
          <ButtonGroup>
            {#if !disabled}
              <Button onclick={() => createSync.start(user)}>
                {m.admin_add_sync_event()}
              </Button>
              <Sync {user} {refresh} />
              <Button onclick={() => disable(user)}>
                {m.admin_disable_user()}
              </Button>
            {/if}
            <Button
              onclick={() => {
                deleteuser?.start(user);
              }}
            >
              {m.admin_delete_user()}
            </Button>
          </ButtonGroup>
        </TableBodyCell>
      </TableBodyRow>
    {/each}
  </Table>
  <ButtonGroup class="p-6">
    <Button onclick={() => createSync.start()}
      >{m.admin_add_sync_event_all()}</Button
    >
    <Button onclick={rescan}>
      {#await promise}
        <Spinner />
      {:then}
        {m.admin_rescan_all()}
      {/await}
    </Button>
  </ButtonGroup>
  <Button onclick={refresh}>{m.action_refresh()}</Button>
{/await}
<CreateSync {refresh} bind:this={createSync} />
<DeleteUser bind:this={deleteuser} {refresh} />
