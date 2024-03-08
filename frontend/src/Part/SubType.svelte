<script lang="ts">
  import { DropdownItem } from "@sveltestrap/sveltestrap";
  import { usages } from "../lib/usage";
  import Usage from "../Usage/Usage.svelte";
  import ReplacePart from "../Attachment/ReplacePart.svelte";
  import AttachPart from "../Attachment/AttachPart.svelte";
  import { Type } from "../lib/types";
  import Menu from "../Widgets/Menu.svelte";
  import ShowAll from "../Widgets/ShowHist.svelte";
  import PartLink from "./PartLink.svelte";
  import { parts, Part } from "../lib/part";
  import type { Attachment } from "../lib/attachment";
  import NewService from "../Service/NewService.svelte";

  export let attachments: Attachment[] = [];
  export let level: number = 0;
  export let prefix = "";
  export let type: Type | undefined = undefined;
  export const hook: Type | undefined = undefined;

  let show_hist = false;
  let attachPart: (p: Part) => void;
  let replacePart: (p: Attachment) => void;
  let newService: (p: Part) => void;
</script>

{#if type == undefined}
  <tr>
    <th scope="col"> <slot /> </th>
    <th scope="col">Name</th>
    <th scope="col">Attached</th>
    <Usage header />
    <th></th>
  </tr>
{:else}
  {#each attachments.map( (att) => ({ att, part: $parts[att.part_id] }), ) as { att, part }, i (att.idx)}
    {#if i == 0}
      <tr>
        <th scope="row" class="text-nowrap">
          {"┃ ".repeat(level)}
          {prefix + " " + type.name}
          {#if attachments.length > 0 || (part && $usages[part.usage].count != $usages[att.usage].count)}
            <ShowAll bind:show_hist />
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
          <td>
            <Menu>
              <DropdownItem on:click={() => newService(part)}>
                Log Service
              </DropdownItem>
              <DropdownItem on:click={() => attachPart(part)}>
                Move part
              </DropdownItem>
              <DropdownItem on:click={() => replacePart(att)}>
                Replace part
              </DropdownItem>
            </Menu>
          </td>
        {:else}
          <th colspan="80" />
        {/if}
      </tr>
    {/if}
    {#if show_hist}
      <tr>
        <th scope="row" class="text-nowrap">
          {"┃ ".repeat(level + 1) + "▶"}
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
        <td>
          {#if part.disposed_at == undefined}
            <Menu>
              <DropdownItem on:click={() => newService(part)}>
                Log Service
              </DropdownItem>
              <DropdownItem on:click={() => attachPart(part)}
                >Attach part
              </DropdownItem>
              <DropdownItem on:click={() => replacePart(att)}>
                Duplicate part
              </DropdownItem>
            </Menu>
          {/if}
        </td>
      </tr>
    {/if}
  {/each}
{/if}
<AttachPart bind:attachPart />
<ReplacePart bind:replacePart />
<NewService bind:newService />
