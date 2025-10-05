<script lang="ts">
  import { Button, Modal } from "flowbite-svelte";
  import { Service } from "../lib/service";
  import { fmtDate } from "../lib/store";

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

<Modal form bind:open {onaction}>
  {#snippet header()}
    Do you really want to delete Service log<br />
    "{service.name}" from
    {fmtDate(service.time)}?
  {/snippet}
  {#snippet footer()}
    <Button onclick={() => (open = false)} color="alternative">Cancel</Button>
    <Button type="submit" value="create" class="float-end">Delete</Button>
  {/snippet}
</Modal>
