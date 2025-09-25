<script lang="ts">
  import { Button, DropdownItem } from "flowbite-svelte";
  import Usage from "../Usage/Usage.svelte";
  import { actions } from "../Widgets/Actions.svelte";
  import Menu from "../Widgets/Menu.svelte";
  import ShowMore from "../Widgets/ShowMore.svelte";
  import type { Attachment } from "../lib/attachment";
  import { parts } from "../lib/part";
  import { Type } from "../lib/types";
  import { usages } from "../lib/usage";
  import PartLink from "./PartLink.svelte";

  export let attachments: Attachment[] = [];
  export let level: number = 0;
  export let prefix = "";
  export let type: Type | undefined = undefined;

  let show_more = false;
</script>

{#if type == undefined}
  <tr>
    <th scope="col"> <slot /> </th>
    <th scope="col"> Name </th>
    <th scope="col"> Attached </th>
    <Usage header />
  </tr>
{:else}
  {#each attachments.map( (att) => ({ att, part: $parts[att.part_id] }), ) as { att, part }, i (att.idx)}
    {#if i == 0}
      <tr>
        <th scope="row" class="text-nowrap">
          {"┃ ".repeat(level)}
          {#if attachments.length > 0 || (part && $usages[part.usage].count != $usages[att.usage].count)}
            <ShowMore bind:show_more title="history" />
          {/if}
          {prefix + " " + type.name}
          {#if att.isAttached()}
            <Menu>
              <DropdownItem on:click={() => $actions.newService(part)}>
                Log Service
              </DropdownItem>
              <DropdownItem on:click={() => $actions.attachPart(part)}>
                Move part
              </DropdownItem>
              <DropdownItem on:click={() => $actions.replacePart(att)}>
                New {type.name}
              </DropdownItem>
            </Menu>
          {:else}
            <Button
              class="float-end"
              size="sm"
              color="light"
              on:click={() => $actions.replacePart(att)}
            >
              add
            </Button>
          {/if}
        </th>
        {#if att.isAttached()}
          <td>
            {#if part}
              <PartLink {part} />
            {:else}
              {att.name}
            {/if}
          </td>
          <td> {att.fmtTime()} </td>
          <Usage id={part.usage} ref={part.id} />
        {:else}
          <th colspan="80"> </th>
        {/if}
      </tr>
    {/if}
    {#if show_more}
      <tr>
        <th scope="row" class="text-nowrap">
          {"┃ ".repeat(level + 1) + "▶"}
          {#if part.disposed_at == undefined}
            <Menu>
              <DropdownItem on:click={() => $actions.newService(part)}>
                Log Service
              </DropdownItem>
              <DropdownItem on:click={() => $actions.attachPart(part)}
                >Attach part
              </DropdownItem>
              <DropdownItem on:click={() => $actions.replacePart(att)}>
                Duplicate part
              </DropdownItem>
            </Menu>
          {/if}
        </th>
        <td>
          {#if part}
            <PartLink {part} />
          {:else}
            {att.name}
          {/if}
        </td>
        <td> {att.fmtTime()} </td>
        <Usage id={att.usage} ref={att.idx} />
      </tr>
    {/if}
  {/each}
{/if}
