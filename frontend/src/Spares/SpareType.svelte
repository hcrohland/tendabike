<script lang="ts">
  import { Button, DropdownItem } from "@sveltestrap/sveltestrap";
  import PartLink from "../Part/PartLink.svelte";
  import Usage from "../Usage/Usage.svelte";
  import { actions } from "../Widgets/Actions.svelte";
  import ShowMore from "../Widgets/ShowMore.svelte";
  import { Attachment, attachments } from "../lib/attachment";
  import { filterValues, type Map } from "../lib/mapable";
  import { parts } from "../lib/part";
  import { fmtDate } from "../lib/store";
  import { Type } from "../lib/types";

  export let type: Type;
  export let date = new Date();
  export let update: (show: boolean) => void;
  export let attachee: number;

  let show_more: boolean = false;

  function attachedTo(
    atts: Map<Attachment>,
    partId: number | undefined,
    time: Date,
  ) {
    let att = filterValues(
      atts,
      (x) => x.part_id === partId && x.isAttached(time),
    ).pop();
    if (att == undefined) return;
    return $parts[att.gear];
  }

  $: subparts = type.parts($parts);
</script>

<tr>
  <th colspan="6" scope="col" class="text-nowrap">
    {#if subparts.length > 0}
      <ShowMore bind:show_more {update} title="attached" />
    {/if}
    &NonBreakingSpace;
    {type.name}s &NonBreakingSpace;
    <Button size="sm" color="light" on:click={() => $actions.newPart(type)}>
      new
    </Button>
  </th>
  <th class="text-nowrap" colspan="80"> </th>
</tr>
{#each subparts.filter((p) => show_more || !(attachedTo($attachments, p.id, date) || p.disposed_at)) as part (part.id)}
  <tr>
    <td class="border-0"> </td>
    <td title={part.vendor + " " + part.model + " " + fmtDate(part.purchase)}>
      <PartLink {part} />
      {#if !part.disposed_at}
        <Button
          color="light"
          class="float-end"
          size="sm"
          on:click={() => $actions.attachPart(part)}
        >
          {#if attachedTo($attachments, part.id, date)}
            Move
          {:else}
            Attach
          {/if}
        </Button>
      {/if}
    </td>
    <Usage id={part.usage} ref={part.id} />
    {#if attachee > 0}
      <td>
        {#if part.disposed_at}
          disposed {fmtDate(part.disposed_at)}
        {:else}
          <PartLink part={attachedTo($attachments, part.id, date)}></PartLink>
        {/if}
      </td>
    {/if}
  </tr>
{/each}
