<script lang="ts">
  import { Button } from "flowbite-svelte";
  import { handleError, myfetch } from "../lib/store";
  import type { User } from "../lib/types";

  export let user: User;
  export let refresh: () => void;

  let promise: Promise<void>, count: number;

  const sync = (id: number) => {
    promise = getdata(id);
  };
  async function getdata(id: number) {
    let data;
    count = 0;
    do {
      data = await myfetch("/strava/sync/" + id).catch(handleError);
      if (!data) break;
      count += data["activities"].length;
    } while (data["activities"].length > 0);
    count = 0;
    refresh();
  }
</script>

<Button onclick={() => sync(user.id)}>
  {#await promise}
    Processed {count} ...
  {:then}
    Process Event Queue
  {/await}
</Button>
