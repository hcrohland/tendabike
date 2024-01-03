<script lang="ts">
  import { Button } from "@sveltestrap/sveltestrap";
  import { handleError, myfetch } from "../store";
  import type { User } from "../types";

  export let user: User;
  export let refresh: () => void;

  let promise, count;

  const sync = (id) => {
    promise = getdata(id);
  };
  async function getdata(id) {
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

<Button on:click={() => sync(user.id)}>
  {#await promise}
    Processed {count} ...
  {:then}
    Process Event Queue
  {/await}
</Button>
