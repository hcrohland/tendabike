<script lang="ts">
  import { ButtonGroup, InputAddon } from "flowbite-svelte";
  import DateTime from "../Widgets/DateTime.svelte";
  import { types } from "../lib/types";
  import { by, filterValues } from "../lib/mapable";
  import { Part } from "../lib/part";
  import { attachments } from "../lib/attachment";
  import SelectPart from "../Widgets/SelectPart.svelte";

  function prevdate(time: Date) {
    let last = filterValues(
      $attachments,
      (a) =>
        (a.attached < time || a.detached < time) &&
        (a.part_id == part.id ||
          (a.gear == gear && a.hook == hook && a.what == part.what)),
    )
      .map((a) => (a.detached < time ? a.detached : a.attached))
      .sort((a, b) => (a < b ? 1 : -1))[0];
    return last || part.purchase;
  }

  export let part: Part;

  let type = part.type();

  export let time = new Date();
  export let gear: number | undefined = undefined;
  export let hook: number | undefined =
    type.hooks.length == 1 ? type.hooks[0] : undefined;
</script>

<div>
  <ButtonGroup>
    <InputAddon>to</InputAddon>
    {#if type.hooks.length > 1}
      <!-- svelte-ignore a11y-autofocus -->
      <select name="hook" class="form-control" required bind:value={hook}>
        <option hidden value={undefined}> -- select one -- </option>
        {#each type.hooks as h}
          <option value={h}>{types[h].name}</option>
        {/each}
      </select>
      <InputAddon>of</InputAddon>
    {/if}
    <SelectPart {type} bind:part={gear} />
  </ButtonGroup>
</div>
<ButtonGroup>
  <InputAddon>at</InputAddon>
  <DateTime bind:date={time} {prevdate} />
</ButtonGroup>
