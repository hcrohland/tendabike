<script module lang="ts">
  import { Activity } from "../lib/activity";

  const modal = $state<{
    open: boolean;
    activity: any;
  }>({
    open: false,
    activity: undefined,
  });

  export const changeActivity = (a: Activity) => {
    modal.open = true;
    modal.activity = { ...a };
  };
</script>

<script lang="ts">
  import { ButtonGroup, InputAddon } from "flowbite-svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { getCategory } from "../lib/types";
  import SelectPart from "../Widgets/SelectPart.svelte";
  import ChangeField from "./ChangeField.svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import * as m from "../../paraglide/messages";

  async function onaction() {
    await new Activity(modal.activity).update();
    modal.open = false;
  }
</script>

{#if modal.activity}
  <Modal bind:open={modal.open} {onaction} size="xs">
    {#snippet header()}
      {m.act_change_header()} <br />
      {modal.activity?.name} <br />
      {m.act_at_time({ time: modal.activity?.start.toLocaleString() })}
    {/snippet}
    <!-- <form on:submit|preventDefault={submit}> -->
    <div>
      <ButtonGroup>
        <InputAddon>{getCategory()!.name}</InputAddon>
        <SelectPart
          type={getCategory()!}
          bind:part={modal.activity.gear}
          none={!modal.activity.gear}
        />
      </ButtonGroup>
    </div>
    <div>
      <ChangeField
        label={m.act_field_climb()}
        bind:field={modal.activity.climb}
      />
      <ChangeField
        label={m.act_field_descend()}
        bind:field={modal.activity.descend}
      />
      <ChangeField
        label={m.act_field_distance()}
        bind:field={modal.activity.distance}
      />
      <ChangeField
        label={m.act_field_time()}
        bind:field={modal.activity.time}
      />
      <ChangeField
        label={m.act_field_duration()}
        bind:field={modal.activity.duration}
      />
    </div>
    {#snippet footer()}
      <Buttons bind:open={modal.open} label={m.action_update()} />
    {/snippet}
  </Modal>
{/if}
