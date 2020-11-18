<script lang="ts">
  import {
    Modal,
    ModalBody,
    ModalHeader,
    ListGroup,
    ListGroupItem,
  } from 'sveltestrap';
  import {checkStatus, parts} from "../store";
  import TimezonePicker from 'svelte-timezone-picker';
  import ModalFooter from './ModalFooter.svelte'

  let timezone  ;

  let isOpen = false;
  let files;
  let result;
  let button;
  const toggle = () => isOpen = false

  export const popup = () => {
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
            parts.updateMap(a[0]); 
            result = {
              good: a[1],
              bad: a[2]
            }
            button = null
          }
        )
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
      <input type="file" bind:files accept="text/csv">
      <br>
      <div class="container"> 
        Timezone of activities: <TimezonePicker bind:timezone />
      </div>
    </ModalBody>
  {/if}
  <ModalFooter {toggle} {disabled} action={sendFile} {button}/>
</Modal>