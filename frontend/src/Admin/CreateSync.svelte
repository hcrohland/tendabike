<script lang="ts">
  import {
    Modal,
    ModalHeader,
    ModalBody,
    FormGroup,
    InputGroup,
    InputGroupText,
    ModalFooter,
  } from "@sveltestrap/sveltestrap";
  import { handleError, myfetch } from "../lib/store";
  import type { User } from "../lib/types";
  import DateTime from "../Widgets/DateTime.svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import Switch from "../Widgets/Switch.svelte";

  export let refresh: () => void;
  let user: User | undefined;
  let date = new Date();
  let isOpen = false;
  let userParam: string;
  let checked = false;
  const toggle = () => (isOpen = false);

  async function submit() {
    await myfetch(
      "/strava/sync?time=" +
        (date.getTime() / 1000).toFixed(0) +
        "&migrate=" +
        checked +
        userParam,
    ).catch(handleError);
    isOpen = false;
    refresh();
  }

  export const createSync = (user?: User) => {
    if (user) {
      userParam = "&user_id=" + user.id;
    } else {
      userParam = "";
    }
    isOpen = true;
  };
</script>

<Modal {isOpen} {toggle} backdrop={false}>
  <ModalHeader {toggle}>
    Create sync Event
    {#if user}
      for {user.firstname} {user.name} ({user.id})
    {/if}
  </ModalHeader>
  <form on:submit|preventDefault={submit}>
    <ModalBody>
      <FormGroup check>
        <InputGroup>
          <InputGroupText>Start</InputGroupText>
          <DateTime maxdate={new Date()} bind:date />
        </InputGroup>
      </FormGroup>
      <Switch bind:checked>Migration</Switch>
    </ModalBody>
    <ModalFooter>
      <Buttons {toggle} label={"Sync"} />
    </ModalFooter>
  </form>
</Modal>
