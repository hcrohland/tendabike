<script lang="ts">
  import { Button } from "flowbite-svelte";
  import { handleError, myfetch } from "../lib/store";
  import type { User } from "../lib/user";
  import * as m from "../../paraglide/messages";

  let { user, refresh }: { user: User; refresh: () => void } = $props();

  let promise = $state<Promise<void>>();

  const sync = (id: number) => {
    promise = getdata(id);
  };
  async function getdata(id: number) {
    // the backend queues the activities onto the per-user executor; the new
    // rows arrive over the stream, so the client just awaits the write
    await myfetch("/strava/sync/" + id).catch(handleError);
    refresh();
  }
</script>

<Button onclick={() => sync(user.id)}>
  {#await promise}
    {m.sync_processed()}
  {:then}
    {m.sync_process_queue()}
  {/await}
</Button>
