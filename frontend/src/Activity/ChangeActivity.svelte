<script lang="ts">
  import { ButtonGroup, InputAddon } from "flowbite-svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { Activity } from "../lib/activity";
  import { category } from "../lib/types";
  import SelectPart from "../Widgets/SelectPart.svelte";
  import ChangeField from "./ChangeField.svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import * as m from "../../paraglide/messages";

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
      {m.act_change_header()} <br />
      {activity?.name} <br />
      {m.act_at_time({ time: activity?.start.toLocaleString() })}
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
      <ChangeField label={m.act_field_climb()} bind:field={activity.climb} />
      <ChangeField
        label={m.act_field_descend()}
        bind:field={activity.descend}
      />
      <ChangeField
        label={m.act_field_distance()}
        bind:field={activity.distance}
      />
      <ChangeField label={m.act_field_time()} bind:field={activity.time} />
      <ChangeField
        label={m.act_field_duration()}
        bind:field={activity.duration}
      />
    </div>
    {#snippet footer()}
      <Buttons bind:open label={m.action_update()} />
    {/snippet}
  </Modal>
{/if}
