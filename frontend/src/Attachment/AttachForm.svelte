<script lang="ts">
  import { InputGroup, InputGroupText } from "flowbite-svelte";
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
  export let disabled = true;

  let type = part.type();

  export let time = new Date();
  export let gear: number | undefined = undefined;
  export let hook: number | undefined =
    type.hooks.length == 1 ? type.hooks[0] : undefined;

  $: disabled = !(hook && gear && types[hook]);
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
    <DateTime bind:date={time} {prevdate} />
  </InputGroup>
</div>
