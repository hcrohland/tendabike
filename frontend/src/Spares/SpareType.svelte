<script lang="ts">
  import {
    Button,
    TableBodyCell,
    TableBodyRow,
    TableHeadCell,
  } from "flowbite-svelte";
  import PartLink from "../Part/PartLink.svelte";
  import Usage from "../Usage/Usage.svelte";
  import ShowMore from "../Widgets/ShowMore.svelte";
  import { Attachment, attachments } from "../lib/attachment";
  import { filterValues, type Map } from "../lib/mapable";
  import { parts } from "../lib/part";
  import { fmtDate } from "../lib/store";
  import { Type } from "../lib/types";
  import { actions } from "../Widgets/Actions.svelte";
  import XsButton from "../Widgets/XsButton.svelte";

  interface Props {
    type: Type;
    date?: any;
    update: (show: boolean) => void;
    attachee: number;
  }

  let { type, date = new Date(), update, attachee }: Props = $props();

  let show_more: boolean = $state(false);

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

  let subparts = $derived(type.parts($parts));
</script>

<tr>
  <TableHeadCell colspan={80} scope="col" class="text-nowrap">
    {#if subparts.length > 0}
      <ShowMore bind:show_more {update} title="attached" />
    {/if}
    {type.name}s &NonBreakingSpace;
    <XsButton onclick={() => $actions.newPart(type)}>New</XsButton>
  </TableHeadCell>
</tr>
{#each subparts.filter((p) => show_more || !(attachedTo($attachments, p.id, date) || p.disposed_at)) as part (part.id)}
  <TableBodyRow class="border-0">
    <TableBodyCell class="border-0"></TableBodyCell>
    <TableBodyCell
      title={part.vendor + " " + part.model + " " + fmtDate(part.purchase)}
    >
      <PartLink {part} />
      {#if !part.disposed_at}
        <XsButton
          class="p-1 cursor-pointer rounded-md float-end"
          onclick={() => $actions.attachPart(part)}
        >
          {#if attachedTo($attachments, part.id, date)}
            Move
          {:else}
            Attach
          {/if}
        </XsButton>
      {/if}
    </TableBodyCell>
    <Usage id={part.usage} ref={part.id} />
    {#if attachee > 0}
      <TableBodyCell>
        {#if part.disposed_at}
          disposed {fmtDate(part.disposed_at)}
        {:else}
          <PartLink part={attachedTo($attachments, part.id, date)}></PartLink>
        {/if}
      </TableBodyCell>
    {/if}
  </TableBodyRow>
{/each}
