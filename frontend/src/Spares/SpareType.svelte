<script lang="ts">
  import {
    type Map,
    filterValues,
    types,
    parts,
    fmtDate,
    attachments,
  } from "../lib/store";
  import { Button, DropdownItem } from "@sveltestrap/sveltestrap";
  import Usage from "../Usage.svelte";
  import AttachPart from "../Actions/AttachPart.svelte";
  import NewPart from "../Actions/NewPart.svelte";
  import { Attachment, Part, Type } from "../lib/types";
  import Menu from "../Widgets/Menu.svelte";
  import ShowAll from "../Widgets/ShowHist.svelte";
  import { link } from "svelte-spa-router";

  export let type: Type;
  export let date = new Date();
  export let update: (show: boolean) => void;
  export let attachee: number;

  let attachPart: (part: Part) => void;
  let newPart: (t: Type) => void;
  let show_all: boolean;

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
    return $parts[att.gear].name + " " + types[att.hook].prefix;
  }

  $: subparts = type.parts($parts);
</script>

<AttachPart bind:attachPart />
<NewPart bind:newPart />

<tr>
  <th colspan="6" scope="col" class="text-nowrap">
    {type.name}s
    {#if subparts.length > 0}
      <ShowAll
        on:toggle={(e) => {
          {
            show_all = e.detail;
            update(show_all);
          }
        }}
      />
    {/if}
  </th>
  <th class="text-nowrap" colspan="80">
    <Button class="badge float-end" on:click={() => newPart(type)}>
      New {type.name}</Button
    >
  </th>
</tr>
{#each subparts.filter((p) => show_all || !(attachedTo($attachments, p.id, date) || p.disposed_at)) as part (part.id)}
  <tr>
    <td class="border-0"></td>
    <td title={part.vendor + " " + part.model + " " + fmtDate(part.purchase)}>
      <a
        href="/part/{part.id}"
        use:link
        style={part.disposed_at ? "text-decoration: line-through;" : ""}
        class="text-reset"
      >
        {part.name}
      </a>
    </td>
    <Usage usage={part} ref={part.id} />
    {#if attachee > 0}
      <td>
        {#if part.disposed_at}
          disposed {fmtDate(part.disposed_at)}
        {:else}
          {attachedTo($attachments, part.id, date) || "-"}
        {/if}
      </td>
    {/if}
    <td>
      {#if !part.disposed_at}
        <Menu>
          <DropdownItem on:click={() => attachPart(part)}>
            {#if attachedTo($attachments, part.id, date)}
              Move
            {:else}
              Attach
            {/if}
            {type.name}
          </DropdownItem>
        </Menu>
      {/if}
    </td>
  </tr>
{/each}
