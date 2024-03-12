<script lang="ts">
  import {
    Button,
    Form,
    FormGroup,
    Offcanvas,
    Spinner,
  } from "@sveltestrap/sveltestrap";
  import { Activity } from "../lib/activity";
  import { category } from "../lib/types";
  import SelectPart from "../Widgets/SelectPart.svelte";
  import ChangeField from "./ChangeField.svelte";

  export const changeActivity = (a: Activity) => {
    isOpen = true;
    activity = new Activity(a);
  };

  let activity: Activity;
  let isOpen = false;

  let spinner = false;

  const toggle = () => (isOpen = !isOpen);

  async function submit() {
    spinner = true;
    await activity.update();
    spinner = false;
    isOpen = false;
  }
</script>

{#if activity}
  <Offcanvas placement="end" {isOpen} {toggle}>
    <div slot="header">
      Change Activity <br />
      {activity?.name} <br />
      at {activity?.start.toLocaleString()}
    </div>
    <Form on:submit={submit}>
      <FormGroup floating label={$category?.name} class="mb-0 mr-sm-2 mb-sm-2">
        <SelectPart
          type={$category}
          bind:part={activity.gear}
          none={!activity.gear}
        />
      </FormGroup>

      <ChangeField label="Climb (m)" bind:field={activity.climb} />
      <ChangeField label="Descend (m)" bind:field={activity.descend} />
      <ChangeField label="Distance (m)" bind:field={activity.distance} />
      <ChangeField label="Time (sec)" bind:field={activity.time} />
      <ChangeField label="Duration (sec)" bind:field={activity.duration} />

      <div class="float-end">
        <Button type="button" on:click={toggle}>Cancel</Button>
        <Button color="primary">
          {#if spinner}
            <Spinner />
          {:else}
            Update
          {/if}
        </Button>
      </div>
    </Form>
  </Offcanvas>
{/if}
