<script lang="ts">
  import { filterValues, by } from "../lib/mapable";
  import { types } from "../lib/types";
  import Usage from "../Usage/Usage.svelte";
  import PartLink from "./PartLink.svelte";
  import { parts } from "../lib/part";
  import { attachments } from "../lib/attachment";
  import {
    DropdownItem,
    Table,
    TableBody,
    TableBodyCell,
    TableBodyRow,
    TableHead,
    TableHeadCell,
  } from "flowbite-svelte";
  import Menu from "../Widgets/Menu.svelte";
  import { actions } from "../Widgets/Actions.svelte";

  interface Props {
    id: number;
  }

  let { id }: Props = $props();

  let atts = $derived(
    filterValues($attachments, (a) => a.part_id == id).sort(by("attached")),
  );
</script>

{#if atts.length > 0}
  <Table hoverable striped>
    <TableHead>
      <TableHeadCell colspan={2} scope="col">Attached to</TableHeadCell>
      <Usage header />
    </TableHead>
    <TableBody>
      {#each atts as att (att.attached)}
        <TableBodyRow>
          <TableBodyCell class="text-nowrap flex justify-between">
            {#if $parts[att.gear]}
              <div>
                <PartLink part={$parts[att.gear]} />
                {types[att.hook].prefix}
              </div>
              <Menu>
                <DropdownItem onclick={() => $actions.deleteAttachment(att)}>
                  Remove
                </DropdownItem>
              </Menu>
            {:else}
              N/A
            {/if}
          </TableBodyCell>
          <TableBodyCell>{att.fmtTime()}</TableBodyCell>
          <Usage id={att.usage} ref={att.idx} />
        </TableBodyRow>
      {/each}
    </TableBody>
  </Table>
{/if}
