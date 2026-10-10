<script lang="ts">
  import { Fileupload, Button } from "flowbite-svelte";
  import { checkStatus, handleError } from "../lib/store";
  import Modal from "../Widgets/Modal.svelte";
  import * as m from "../../paraglide/messages";

  let files: FileList | undefined = $state();
  /// The upload answers 204 (no body): the new activities ride the stream
  /// frame, so the client keeps only a plain success state (failures go to
  /// the global banner via `handleError`).
  let success = $state(false);

  interface Props {
    open: boolean;
  }

  let { open = $bindable() }: Props = $props();

  let disabled = $derived(!(files && files[0]));

  function reset() {
    files = undefined;
    open = false;
    success = false;
  }

  async function sendFile() {
    // The endpoint takes the raw CSV text (not a JSON body), so the handler
    // uses a plain fetch with `checkStatus` (204 → `null`, 401 → redirect)
    // instead of `myfetch`, which would JSON-encode the body.
    var body = files && (await files[0].text());
    return fetch("/api/activ/descend", {
      method: "POST",
      credentials: "include",
      body,
    })
      .then(checkStatus)
      .then(() => {
        success = true;
        files = undefined;
      })
      .catch(handleError);
  }
</script>

<Modal bind:open title={m.garmin_upload_title()}>
  {#if success}
    {m.garmin_sync_success()}
  {:else}
    <Fileupload bind:files accept="text/csv" title={m.garmin_upload_hint()} />
    <br />
  {/if}
  {#snippet footer()}
    <div class="flex justify-end w-full gap-2">
      {#if !success}
        <Button onclick={sendFile} color="dark" class="border" {disabled}>
          {m.action_synchronize()}
        </Button>
        <Button onclick={reset} color="alternative">{m.action_cancel()}</Button>
      {:else}
        <Button onclick={reset}>{m.action_ok()}</Button>
      {/if}
    </div>
  {/snippet}
</Modal>
