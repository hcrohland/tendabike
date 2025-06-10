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
  import { by, filterValues } from "../lib/mapable";
  import { activities } from "../lib/activity";

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

  export const createSync = (u?: User) => {
    user = u;
    if (u) {
      userParam = "&user_id=" + u.id;
    } else {
      userParam = "";
    }
    isOpen = true;
  };

  function prevdate(date: Date) {
    let prev = filterValues(
      $activities,
      (a) => a.user_id == user?.id && a.start < date,
    ).sort(by("start"))[0];
    return prev ? prev.start : date;
  }
</script>

<Modal {isOpen} {toggle}>
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
          <DateTime bind:date prevdate={user ? prevdate : undefined} />
        </InputGroup>
      </FormGroup>
      <Switch bind:checked>Migration</Switch>
    </ModalBody>
    <ModalFooter>
      <Buttons {toggle} label={"Sync"} />
    </ModalFooter>
  </form>
</Modal>
