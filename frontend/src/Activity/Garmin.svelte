<script lang="ts">
  import {
    Modal,
    Listgroup,
    ListgroupItem,
    Fileupload,
    Button,
  } from "flowbite-svelte";
  import { checkStatus, handleError } from "../lib/store";
  import { parts } from "../lib/part";
  import { attachments } from "../lib/attachment";

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
        parts.updateMap(a[0].parts);
        attachments.updateMap(a[0].attachments);
        result = {
          good: a[1],
          bad: a[2],
        };
        files = undefined;
      })
      .catch(handleError);
  }
</script>

<Modal bind:open title="Upload Garmin activities file">
  {#if result}
    {#if result.good.length > 0}
      Synchronized {result.good.length} activities.
    {/if}
    {#if result.bad.length > 0}
      <br /><br />Could not match the following {result.bad.length} activities:
      <Listgroup>
        {#each result.bad as r}
          <ListgroupItem>{r}</ListgroupItem>
        {/each}
      </Listgroup>
    {/if}
  {:else}
    <Fileupload
      bind:files
      accept="text/csv"
      title="Upload a CSV file exported from Garmin connect activities. 
It will match activities based on the start time. 
If there is no match it will skip the activity.
You can upload multiple times."
    />
    <br />
  {/if}
  {#snippet footer()}
    {#if !result}
      <Button onclick={sendFile} {disabled}>Synchronize</Button>
      <Button onclick={reset}>Cancel</Button>
    {:else}
      <Button onclick={reset}>Ok</Button>
    {/if}
  {/snippet}
</Modal>
