<script lang="ts">
  import {
    Modal,
    ModalBody,
    ModalHeader,
    ListGroup,
    ListGroupItem,
    Input,
  } from "@sveltestrap/sveltestrap";
  import { checkStatus, handleError } from "../lib/store";
  import MyFooter from "../Widgets/MyFooter.svelte";
  import { parts } from "../lib/part";
  import { attachments } from "../lib/attachment";

  let isOpen = false;
  let files: FileList | undefined;
  let result: { good: string[]; bad: string[] } | undefined;
  let label: string | undefined;
  const toggle = () => (isOpen = false);

  export const garmin = () => {
    files = undefined;
    result = undefined;
    label = "Synchronize";
    isOpen = true;
  };

  $: disabled = !(files && files[0]);

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
        label = undefined;
      })
      .catch(handleError);
  }
</script>

<Modal {isOpen} {toggle} scrollable>
  <ModalHeader {toggle}>Upload Garmin activities file</ModalHeader>
  {#if result}
    <ModalBody>
      {#if result.good.length > 0}
        Synchronized {result.good.length} activities.
      {/if}
      {#if result.bad.length > 0}
        <br /><br />Could not match the following {result.bad.length} activities:
        <ListGroup>
          {#each result.bad as r}
            <ListGroupItem>{r}</ListGroupItem>
          {/each}
        </ListGroup>
      {/if}
    </ModalBody>
  {:else}
    <ModalBody>
      <Input
        type="file"
        bind:files
        accept="text/csv"
        title="Upload a CSV file exported from Garmin connect activities. 
It will match activities based on the start time. 
If there is no match it will skip the activity.
You can upload multiple times."
      />
      <br />
    </ModalBody>
  {/if}
  <MyFooter {toggle} {disabled} action={sendFile} {label} />
</Modal>
