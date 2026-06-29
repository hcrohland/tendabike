<script lang="ts">
  import PartLink from "../Part/PartLink.svelte";
  import UsageChips from "../Usage/UsageChips.svelte";
  import ShowMore from "../Widgets/ShowMore.svelte";
  import { DropdownItem } from "flowbite-svelte";
  import Menu from "../Widgets/Menu.svelte";
  import XsButton from "../Widgets/XsButton.svelte";
  import { Attachment, attachments } from "../lib/attachment";
  import { filterValues, type Map } from "../lib/mapable";
  import { parts } from "../lib/part";
  import { fmtDate } from "../lib/store";
  import { Type } from "../lib/types";
  import { actions } from "../Widgets/Actions.svelte";
  import { shop } from "../lib/shop";

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

  let subparts = $derived(
    type.parts($parts).filter((p) => ($shop ? p.shop == $shop.id : true)),
  );
  let subshow = $derived(
    subparts.filter(
      (p) =>
        show_more || (!p.disposed_at && !attachedTo($attachments, p.id, date)),
    ),
  );
</script>

<div class="flex flex-col gap-2">
  <!-- Type header -->
  <div class="flex items-center justify-between gap-2 px-1">
    <div class="flex items-center gap-2">
      {#if subparts.length > 0}
        <ShowMore bind:show_more {update} title="attached" />
      {/if}
      <span
        class="text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400"
      >
        {type.name}s
      </span>
    </div>
    <XsButton onclick={() => $actions.newPart(type)}>New</XsButton>
  </div>

  <!-- Part cards -->
  {#each subshow as part (part.id)}
    <div
      class={"rounded-lg border border-gray-200 dark:border-gray-600 p-3 " +
        (part.disposed_at
          ? "bg-surface-2 opacity-70"
          : attachedTo($attachments, part.id, date)
            ? "bg-surface-2"
            : "bg-surface-1")}
    >
      <!-- Name + menu -->
      <div class="flex items-center justify-between gap-2">
        <div class="min-w-0">
          <span class="font-medium text-sm">
            <PartLink {part} />
          </span>
          <span
            class="text-xs text-gray-500 dark:text-gray-400 ml-1"
            title={part.vendor +
              " " +
              part.model +
              " " +
              fmtDate(part.purchase)}
          >
            · {part.vendor}
            {part.model} · {fmtDate(part.purchase)}
          </span>
        </div>
        {#if !part.disposed_at}
          <div class="shrink-0">
            <Menu>
              <DropdownItem onclick={() => $actions.attachPart(part)}>
                {attachedTo($attachments, part.id, date) ? "Move" : "Attach"}
              </DropdownItem>
              {#if part.attachments($attachments).length == 0}
                <DropdownItem onclick={() => $actions.deletePart(part)}>
                  Delete
                </DropdownItem>
              {:else}
                <DropdownItem onclick={() => $actions.disposePart(part)}>
                  Dispose
                </DropdownItem>
              {/if}
            </Menu>
          </div>
        {/if}
      </div>

      <!-- Stats -->
      <UsageChips
        id={part.usage}
        ref={part.id}
        light={!part.disposed_at && !attachedTo($attachments, part.id, date)}
      />

      <!-- Attached to -->
      {#if attachee > 0}
        <div class="mt-2 text-xs text-gray-500 dark:text-gray-400">
          {#if part.disposed_at}
            disposed {fmtDate(part.disposed_at)}
          {:else}
            {@const attachedPart = attachedTo($attachments, part.id, date)}
            {#if attachedPart}
              Attached to:
              <span class="text-xs text-gray-500 dark:text-gray-200 ml-1">
                <PartLink part={attachedPart} />
              </span>
            {/if}
          {/if}
        </div>
      {/if}
    </div>
  {/each}
</div>
