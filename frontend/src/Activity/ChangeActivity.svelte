<script lang="ts">
  import { FormGroup, Offcanvas } from "@sveltestrap/sveltestrap";
  import { Activity } from "../lib/activity";
  import { category } from "../lib/types";
  import SelectPart from "../Widgets/SelectPart.svelte";
  import ChangeField from "./ChangeField.svelte";
  import Buttons from "../Widgets/Buttons.svelte";

  export const changeActivity = (a: Activity) => {
    isOpen = true;
    activity = new Activity(a);
  };

  let activity: Activity;
  let isOpen = false;

  const toggle = () => (isOpen = !isOpen);

  async function submit() {
    await activity.update();
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
    <form on:submit|preventDefault={submit}>
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
        <Buttons {toggle} label="Update" />
      </div>
    </form>
  </Offcanvas>
{/if}
