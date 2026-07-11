<script lang="ts">
  import { Service } from "../lib/service";
  import { fmtDate } from "../lib/store";
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { m } from "../../paraglide/messages";

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
    {m.deleteservice_header({
      name: service.name,
      date: fmtDate(service.time),
    })}
  {/snippet}
  {#snippet footer()}
    <Buttons bind:open label={m.action_delete()} />
  {/snippet}
</Modal>
