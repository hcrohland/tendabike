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
  import { updateSummary } from "../lib/user";

  let files: FileList | undefined = $state();
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
    var body = files && (await files[0].text());
    return fetch("/api/activ/descend", {
      method: "POST",
      credentials: "include",
      body,
    })
      .then(checkStatus)
      .then((a) => {
        updateSummary(a[0]);
        result = {
          good: a[1],
          bad: a[2],
        };
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
    {#if !result}
      <Button onclick={sendFile} {disabled}>{m.action_synchronize()}</Button>
      <Button onclick={reset}>{m.action_cancel()}</Button>
    {:else}
      <Button onclick={reset}>{m.action_ok()}</Button>
    {/if}
  {/snippet}
</Modal>
