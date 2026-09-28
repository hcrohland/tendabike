<script module lang="ts">
  import { Service } from "../lib/service";

  const modal = $state<{
    open: boolean;
    service: Service;
  }>({
    open: false,
    service: new Service({}),
  });

  export function start(s: Service) {
    modal.service = s;
    modal.open = true;
  }
</script>

<script lang="ts">
  import { fmtDate } from "../lib/store";
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { m } from "../../paraglide/messages";

  async function onaction() {
    await modal.service.delete();
    modal.open = false;
  }
</script>

<Modal bind:open={modal.open} {onaction}>
  {#snippet header()}
    {m.deleteservice_header({
      name: modal.service.name,
      date: fmtDate(modal.service.time),
    })}
  {/snippet}
  {#snippet footer()}
    <Buttons bind:open={modal.open} label={m.action_delete()} />
  {/snippet}
</Modal>
