<script lang="ts">
  import {
    DropdownItem,
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
  import Menu from "../Widgets/Menu.svelte";

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
  let subshow = $derived(
    subparts.filter(
      (p) =>
        show_more || !(attachedTo($attachments, p.id, date) || p.disposed_at),
    ),
  );
</script>

<TableBodyRow>
  <TableHeadCell colspan={80} scope="col" class="text-nowrap">
    {#if subparts.length > 0}
      <ShowMore bind:show_more {update} title="attached" />
    {/if}
    {type.name}s &NonBreakingSpace;
    <XsButton onclick={() => $actions.newPart(type)}>New</XsButton>
  </TableHeadCell>
</TableBodyRow>
{#each subshow as part, i (part.id)}
  <TableBodyRow>
    <TableHeadCell scope="row" class="ps-4">
      {#if i == subshow.length - 1}
        ┗
      {:else}
        ┃
      {/if}
    </TableHeadCell>

    <TableBodyCell
      title={part.vendor + " " + part.model + " " + fmtDate(part.purchase)}
      class="flex justify-between"
    >
      <PartLink {part} />
      {#if !part.disposed_at}
        <Menu>
          <DropdownItem onclick={() => $actions.attachPart(part)}>
            {#if attachedTo($attachments, part.id, date)}
              Move
            {:else}
              Attach
            {/if}
          </DropdownItem>
        </Menu>
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
