<script lang="ts">
  import { handleError, myfetch } from "../lib/store";
  import type { User } from "../lib/user";
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";
  import * as m from "../../paraglide/messages";

  let user = $state<any>();

  let { refresh } = $props();
  let open = $state(false);

  async function onaction() {
    myfetch("/strava/delete/" + user.id, "DELETE")
      .then(refresh)
      .catch(handleError);
    open = false;
  }

  export const start = (p: User) => {
    user = p;
    open = true;
    refresh = refresh;
  };
</script>

<Modal size="xs" bind:open {onaction}>
  {#snippet header()}
    {m.admin_delete_user_confirm({ name: `${user.firstname} ${user.name}` })}
  {/snippet}
  {#snippet footer()}
    <Buttons bind:open label={m.action_delete()} />
  {/snippet}
</Modal>
