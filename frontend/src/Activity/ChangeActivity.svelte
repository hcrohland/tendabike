<script lang="ts">
  import { ButtonGroup, InputAddon } from "flowbite-svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { Activity } from "../lib/activity";
  import { category } from "../lib/types";
  import SelectPart from "../Widgets/SelectPart.svelte";
  import ChangeField from "./ChangeField.svelte";
  import Buttons from "../Widgets/Buttons.svelte";

  export const start = (a: Activity) => {
    open = true;
    activity = { ...a };
  };

  let activity: any;
  let open = false;

  async function onaction() {
    await new Activity(activity).update();
    open = false;
  }
</script>

{#if activity}
  <Modal bind:open {onaction} size="xs">
    {#snippet header()}
      Change Activity <br />
      {activity?.name} <br />
      at {activity?.start.toLocaleString()}
    {/snippet}
    <!-- <form on:submit|preventDefault={submit}> -->
    <div>
      <ButtonGroup>
        <InputAddon>{$category.name}</InputAddon>
        <SelectPart
          type={$category}
          bind:part={activity.gear}
          none={!activity.gear}
        />
      </ButtonGroup>
    </div>
    <div>
      <ChangeField label="Climb (m)" bind:field={activity.climb} />
      <ChangeField label="Descend (m)" bind:field={activity.descend} />
      <ChangeField label="Distance (m)" bind:field={activity.distance} />
      <ChangeField label="Time (sec)" bind:field={activity.time} />
      <ChangeField label="Duration (sec)" bind:field={activity.duration} />
    </div>
    {#snippet footer()}
      <Buttons bind:open label="Update" />
    {/snippet}
  </Modal>
{/if}
