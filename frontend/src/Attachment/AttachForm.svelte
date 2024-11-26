<script lang="ts">
  import { InputGroup, InputGroupText } from "@sveltestrap/sveltestrap";
  import DateTime from "../Widgets/DateTime.svelte";
  import { types } from "../lib/types";
  import { by, filterValues } from "../lib/mapable";
  import { Part } from "../lib/part";
  import { AttEvent, attachments } from "../lib/attachment";
  import SelectPart from "../Widgets/SelectPart.svelte";

  function lastDetach(part: Part) {
    let last = filterValues($attachments, (a) => a.part_id == part.id).sort(
      by("attached"),
    )[0];

    if (last) {
      return last.isDetached() ? last.detached : last.attached;
    } else {
      return part.purchase;
    }
  }

  export let attach: AttEvent;
  export let part: Part;
  export let disabled = true;

  let type = part.type();

  let date = new Date();
  let prevdate: Date = lastDetach(part);
  let gear: number | undefined = undefined;
  let hook: number | undefined =
    type.hooks.length == 1 ? type.hooks[0] : undefined;

  $: if (hook && gear && types[hook]) {
    disabled = false;
    attach = new AttEvent(part.id, date, gear, hook);
  } else {
    disabled = true;
  }
</script>

<div class="form-inline">
  <InputGroup class="mb-0 mr-sm-2 mb-sm-2">
    <InputGroupText>to</InputGroupText>
    {#if type.hooks.length > 1}
      <!-- svelte-ignore a11y-autofocus -->
      <select name="hook" class="form-control" required bind:value={hook}>
        <option hidden value={undefined}> -- select one -- </option>
        {#each type.hooks as h}
          <option value={h}>{types[h].name}</option>
        {/each}
      </select>
      <InputGroupText>of</InputGroupText>
    {/if}
    <SelectPart {type} bind:part={gear} />
  </InputGroup>
  <InputGroup class="mb-0 mr-sm-2 mb-sm-2">
    <InputGroupText>at</InputGroupText>
    <DateTime bind:date {prevdate} />
  </InputGroup>
</div>
