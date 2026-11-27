<script lang="ts">
  import { Button, ButtonGroup, Spinner } from "flowbite-svelte";
  import { handleError, myfetch } from "../lib/utils";
  import { type AdminRow, type User } from "../lib/user.svelte";
  import AdminCard from "./AdminCard.svelte";
  import CreateSync from "./CreateSync.svelte";
  import DeleteUser from "./DeleteUser.svelte";
  import * as m from "../../paraglide/messages";

  let promise: Promise<void>, createSync: any;
  let request: Promise<AdminRow[]> | AdminRow[] = [];

  function refresh() {
    request = myfetch("/api/user/all").catch(handleError);
  }

  let deleteuser = { start: (_: User) => {} };

  function rescan() {
    // The rescan stops every live stream after its commit: each
    // client — this one included — re-hydrates via the native reconnect +
    // catch-up snapshot, so no manual refresh is needed here.
    promise = myfetch("/api/activ/rescan").catch(handleError);
  }

  refresh();
</script>

{#await request}
  <Spinner size="16" class="justify-center" />
{:then list: AdminRow[]}
  <div class="flex flex-col gap-1 sm:gap-2 max-w-4xl mx-auto">
    {#each list.sort((a, b) => a.user.id - b.user.id) as row (row.user.id)}
      <AdminCard
        {row}
        {refresh}
        createSync={(u: User) => createSync.start(u)}
        deleteUser={(u: User) => deleteuser?.start(u)}
      />
    {/each}
  </div>
  <ButtonGroup class="p-6">
    <Button color="secondary" onclick={() => createSync.start()}
      >{m.admin_add_sync_event_all()}</Button
    >
    <Button color="secondary" onclick={rescan}>
      {#await promise}
        <Spinner />
      {:then}
        {m.admin_rescan_all()}
      {/await}
    </Button>
  </ButtonGroup>
  <Button color="secondary" onclick={refresh}>{m.action_refresh()}</Button>
{/await}
<CreateSync {refresh} bind:this={createSync} />
<DeleteUser bind:this={deleteuser} {refresh} />
