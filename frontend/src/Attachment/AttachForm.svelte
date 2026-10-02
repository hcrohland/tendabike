<script lang="ts">
  import { ButtonGroup, InputAddon, Select } from "flowbite-svelte";
  import DateTime from "../Widgets/DateTime.svelte";
  import { types } from "../lib/types";
  import { Part } from "../lib/part";
  import { prev_attach_time } from "../lib/attachment";
  import SelectPart from "../Widgets/SelectPart.svelte";
  import { m } from "../../paraglide/messages";

  let {
    part,
    time = $bindable(),
    gear = $bindable(),
    hook = $bindable(),
  }: {
    part: Part;
    time: Date;
    gear?: number | undefined;
    hook?: number | undefined;
  } = $props();

  let type = $derived(part.type());

  // svelte-ignore state_referenced_locally
  if (hook === undefined && type.hooks.length === 1) {
    hook = type.hooks[0];
  }

  // the boundaries of this part's rows and the slot's rows; the walk
  // button disables at the oldest boundary instead of jumping to the
  // part's purchase date (the clock button restores a walkable state)
  function prevdate(time: Date) {
    return prev_attach_time(time, part.id, gear, hook, part.what);
  }
</script>

<div>
  <ButtonGroup>
    <InputAddon>{m.attachform_to()}</InputAddon>
    {#if type.hooks.length > 1}
      <Select
        name="hook"
        required
        bind:value={hook}
        placeholder={m.attachform_select_part()}
        classes={{ select: "rounded-none" }}
      >
        {#each type.hooks as h}
          <option value={h}>{types[h].localizedName()}</option>
        {/each}
      </Select>
      <InputAddon>{m.attachform_of()}</InputAddon>
    {/if}
    <SelectPart {type} bind:part={gear} />
  </ButtonGroup>
</div>
<ButtonGroup>
  <InputAddon>{m.attachform_at()}</InputAddon>
  <DateTime bind:date={time} {prevdate} />
</ButtonGroup>
