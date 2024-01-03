<script lang="ts">
  import {
    Modal, ModalBody, ModalHeader,
    ListGroup, ListGroupItem, Input,
    InputGroup, InputGroupText, Label
  } from '@sveltestrap/sveltestrap';
  import {attachments, checkStatus, handleError, parts} from "../store";
  import TZPicker from '../Widgets/TZPicker.svelte';
  import ModalFooter from './ModalFooter.svelte'

  let timezone: string;

  let isOpen = false;
  let files;
  let result;
  let button;
  const toggle = () => isOpen = false

  export const garmin = () => {
    files = undefined; 
    timezone = undefined;
    result = undefined;
    button = "Synchronize";
    isOpen = true;
  }

  $: disabled = !(files && files[0])

  async function sendFile () {
    var body = await files[0].text();
    return fetch('/activ/descend?tz=' + timezone, {
            method: 'POST',
            credentials: 'include',
            body
        })
        .then(checkStatus)
        .then((a) => {
          parts.updateMap(a[0].parts); 
          attachments.updateMap(a[0].attachments);
          result = {
            good: a[1],
            bad: a[2]
          }
          button = null
        }
        )
        .catch(handleError)
  };
</script>

<Modal {isOpen} {toggle} backdrop={false} transitionOptions={{}}>
  <ModalHeader {toggle}>Upload Garmin activities file</ModalHeader>
  {#if result}
    <ModalBody>
      {#if result.good.length > 0}
        Synchronized {result.good.length} activities. 
      {/if}
      {#if result.bad.length > 0}
        <br><br>Could not match the following {result.bad.length} activities:
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
If there is no match it will skip the activity - maybe it was in another timezone? 
You can upload multiple times" 
      />
      <br>
      <InputGroup class="mb-0 mr-sm-2 mb-sm-2">
        <InputGroupText>Timezone of activities: </InputGroupText>
        <TZPicker bind:timezone />
      </InputGroup>
      </ModalBody>
  {/if}
  <ModalFooter {toggle} {disabled} action={sendFile} {button}/>
</Modal>