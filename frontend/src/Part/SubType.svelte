<script lang="ts">
  import {
    Button,
    DropdownItem,
    TableBodyCell,
    TableBodyRow,
    TableHeadCell,
  } from "flowbite-svelte";
  import Usage from "../Usage/Usage.svelte";
  import Menu from "../Widgets/Menu.svelte";
  import ShowMore from "../Widgets/ShowMore.svelte";
  import type { Attachment } from "../lib/attachment";
  import { parts } from "../lib/part";
  import { Type } from "../lib/types";
  import { usages } from "../lib/usage";
  import PartLink from "./PartLink.svelte";
  import { actions } from "../Widgets/Actions.svelte";

  export let attachments: Attachment[] = [];
  export let level: number = 0;
  export let prefix = "";
  export let type: Type | undefined = undefined;

  let show_more = false;
</script>

{#if type == undefined}
  <TableHeadCell scope="col"><slot /></TableHeadCell>
  <TableHeadCell scope="col">Name</TableHeadCell>
  <TableHeadCell scope="col">Attached</TableHeadCell>
  <Usage header />
{:else}
  {#each attachments.map( (att) => ({ att, part: $parts[att.part_id] }), ) as { att, part }, i (att.idx)}
    {#if i == 0}
      <tr>
        <TableHeadCell scope="row" class="text-nowrap flex justify-between">
          <div>
            {"┃ ".repeat(level)}
            {#if attachments.length > 0 || (part && $usages[part.usage].count != $usages[att.usage].count)}
              <ShowMore bind:show_more title="history" />
            {/if}
            {prefix + " " + type.name}
          </div>
        </TableHeadCell>
        {#if att.isAttached()}
          <TableBodyCell>
            <div class="text-nowrap flex justify-between">
              <div>
                {#if part}
                  <PartLink {part} />
                {:else}
                  {att.name}
                {/if}
              </div>
              <Menu>
                <DropdownItem onclick={() => $actions.newService(part)}>
                  Log Service
                </DropdownItem>
                <DropdownItem onclick={() => $actions.attachPart(part)}>
                  Move part
                </DropdownItem>
                <DropdownItem onclick={() => $actions.replacePart(att)}>
                  New {type.name}
                </DropdownItem>
              </Menu>
            </div>
          </TableBodyCell>
          <TableBodyCell>{att.fmtTime()}</TableBodyCell>
          <Usage id={part.usage} ref={part.id} />
        {:else}
          <TableBodyCell>
            <div class="flex justify-end">
              <Button
                class="p-1 border-0 cursor-pointer"
                size="xs"
                color="alternative"
                onclick={() => $actions.replacePart(att)}
              >
                add
              </Button>
            </div>
          </TableBodyCell>
          <TableBodyCell colspan={80}></TableBodyCell>
        {/if}
      </tr>
    {/if}
    {#if show_more}
      <TableBodyRow>
        <TableHeadCell scope="row">
          {"┃ ".repeat(level + 1) + "▶"}
        </TableHeadCell>
        <TableBodyCell>
          <div class="text-nowrap flex justify-between">
            <div>
              {#if part}
                <PartLink {part} />
              {:else}
                {att.name}
              {/if}
            </div>
            {#if part.disposed_at == undefined}
              <Menu>
                <DropdownItem onclick={() => $actions.newService(part)}>
                  Log Service
                </DropdownItem>
                <DropdownItem onclick={() => $actions.attachPart(part)}>
                  Attach part
                </DropdownItem>
                <DropdownItem onclick={() => $actions.replacePart(att)}>
                  Duplicate part
                </DropdownItem>
              </Menu>
            {/if}
          </div>
        </TableBodyCell>
        <TableBodyCell>{att.fmtTime()}</TableBodyCell>
        <Usage id={att.usage} ref={att.idx} />
      </TableBodyRow>
    {/if}
  {/each}
{/if}
