<script lang="ts">
  import {
    Listgroup,
    ListgroupItem,
    Fileupload,
    Button,
  } from "flowbite-svelte";
  import { checkStatus, handleError } from "../lib/store";
  import Modal from "../Widgets/Modal.svelte";
  import * as m from "../../paraglide/messages";

  let files: FileList | undefined = $state();
  /// The match report of the upload (`200 + {good, bad}`): the rows the
  /// domain matched against the user's activities and the rows it skipped.
  /// It is rendered, never merged — the matched activities' state rides the
  /// stream frame the executor publishes (the spec §6.2 deviation recorded
  /// on issue #446).
  let result: { good: string[]; bad: string[] } | undefined = $state();

  interface Props {
    open: boolean;
  }

  let { open = $bindable() }: Props = $props();

  let disabled = $derived(!(files && files[0]));

  function reset() {
    files = undefined;
    open = false;
    result = undefined;
  }

  async function sendFile() {
    // The endpoint takes the raw CSV text (not a JSON body), so the handler
    // uses a plain fetch with `checkStatus` (204 → `null`, 401 → redirect)
    // instead of `myfetch`, which would JSON-encode the body. The 200 body
    // is the match report; the state change rides the stream frame.
    var body = files && (await files[0].text());
    return fetch("/api/activ/descend", {
      method: "POST",
      credentials: "include",
      body,
    })
      .then(checkStatus)
      .then((a) => {
        result = a;
        files = undefined;
      })
      .catch(handleError);
  }
</script>

<Modal bind:open title={m.garmin_upload_title()}>
  {#if result}
    {#if result.good.length > 0}
      {m.garmin_sync_success({ count: result.good.length })}
    {/if}
    {#if result.bad.length > 0}
      <br />
      {m.garmin_sync_failed({ count: result.bad.length })}
      <br />
      <Listgroup>
        {#each result.bad as r}
          <ListgroupItem>{r}</ListgroupItem>
        {/each}
      </Listgroup>
    {/if}
  {:else}
    <Fileupload bind:files accept="text/csv" title={m.garmin_upload_hint()} />
    <br />
  {/if}
  {#snippet footer()}
    <div class="flex justify-end w-full gap-2">
      {#if !result}
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
