<script lang="ts">
  import { Service } from "../lib/service";
  import { fmtDate } from "../lib/store";
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";

  let service = $state(new Service({}));
  let open = $state(false);

  export function start(s: Service) {
    service = s;
    open = true;
  }

  async function onaction() {
    await service.delete();
    open = false;
  }
</script>

<Modal bind:open {onaction}>
  {#snippet header()}
    Do you really want to delete Service log<br />
    "{service.name}" from
    {fmtDate(service.time)}?
  {/snippet}
  {#snippet footer()}
    <Buttons bind:open label="Delete" />
  {/snippet}
</Modal>
