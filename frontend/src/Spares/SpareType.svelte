<script lang="ts">
  import { fmtDate } from "../lib/store";
  import { Button, DropdownItem } from "@sveltestrap/sveltestrap";
  import Usage from "../Usage/Usage.svelte";
  import AttachPart from "../Attachment/AttachPart.svelte";
  import NewPart from "../Part/NewPart.svelte";
  import { types, Type } from "../lib/types";
  import Menu from "../Widgets/Menu.svelte";
  import ShowAll from "../Widgets/ShowHist.svelte";
  import PartLink from "../Part/PartLink.svelte";
  import { type Map, filterValues } from "../lib/mapable";
  import { Part, parts } from "../lib/part";
  import { attachments, Attachment } from "../lib/attachment";

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
    {type.name}s &NonBreakingSpace;
    <Button size="sm" color="light" on:click={() => newPart(type)}>add</Button>
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
  <th class="text-nowrap" colspan="80"> </th>
</tr>
{#each subparts.filter((p) => show_all || !(attachedTo($attachments, p.id, date) || p.disposed_at)) as part (part.id)}
  <tr>
    <td class="border-0"></td>
    <td title={part.vendor + " " + part.model + " " + fmtDate(part.purchase)}>
      <PartLink {part} />
    </td>
    <Usage id={part.usage} ref={part.id} />
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
