<script lang="ts">
  import PartLink from "../Part/PartLink.svelte";
  import UsageChips from "../Usage/UsageChips.svelte";
  import ShowMore from "../Widgets/ShowMore.svelte";
  import { DropdownItem } from "flowbite-svelte";
  import Menu from "../Widgets/Menu.svelte";
  import XsButton from "../Widgets/XsButton.svelte";
  import { mounted_on } from "../lib/attachment";
  import { fmtDate } from "../lib/store";
  import { Type } from "../lib/types";
  import { newPart } from "../Part/NewPart.svelte";
  import { attachPart } from "../Attachment/AttachPart.svelte";
  import { deletePart } from "../Part/DeletePart.svelte";
  import { disposePart } from "../Part/DisposePart.svelte";
  import { getShop } from "../lib/shop";
  import * as m from "../../paraglide/messages";

  interface Props {
    type: Type;
    date?: any;
    update: (show: boolean) => void;
    attachee: number;
  }

  let { type, date = new Date(), update, attachee }: Props = $props();

  let show_more: boolean = $state(false);

  let subparts = $derived(
    type.parts().filter((p) => (getShop() ? p.shop == getShop()!.id : true)),
  );
  let subshow = $derived(
    subparts.filter(
      (p) => show_more || (!p.disposed_at && !mounted_on(p.id, date)),
    ),
  );
</script>

<div
  class="flex flex-col gap-1 rounded-lg border bg-surface-2 border-border-subtle p-3"
>
  <!-- Type header -->
  <div class="flex items-center justify-between gap-2 pb-2">
    <div class="flex items-center gap-2">
      {#if subparts.length > 0}
        <ShowMore bind:show_more {update} title={m.sparetype_attached()} />
      {/if}
      <span class="text-xs uppercase tracking-wide text-text-2">
        {type.localizedName()}
      </span>
    </div>
    <XsButton onclick={() => newPart(type)}>{m.action_new()}</XsButton>
  </div>

  <!-- Part cards -->
  {#each subshow as part (part.id)}
    <div
      class={"rounded-lg border p-3 " +
        (part.disposed_at
          ? "bg-surface-2 opacity-70 border-border-strong"
          : mounted_on(part.id, date)
            ? "bg-surface-2 border-gray-strong"
            : "bg-surface-1 border-border-subtle")}
    >
      <!-- Name + menu -->
      <div class="flex items-center justify-between gap-2">
        <div class="min-w-0">
          <span class="font-medium text-sm">
            <PartLink {part} />
          </span>
          <span
            class="text-xs text-text-1 ml-1"
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
              <DropdownItem onclick={() => attachPart(part)}>
                {mounted_on(part.id, date)
                  ? m.action_move()
                  : m.action_attach()}
              </DropdownItem>
              {#if part.attachments().length == 0}
                <DropdownItem onclick={() => deletePart(part)}>
                  {m.action_delete()}
                </DropdownItem>
              {:else}
                <DropdownItem onclick={() => disposePart(part)}>
                  {m.action_dispose()}
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
        light={!part.disposed_at && !mounted_on(part.id, date)}
      />

      <!-- Attached to -->
      {#if attachee > 0}
        <div class="mt-2 text-xs text-text-1">
          {#if part.disposed_at}
            {m.sparetype_disposed()} {fmtDate(part.disposed_at)}
          {:else}
            {@const attachedPart = mounted_on(part.id, date)}
            {#if attachedPart}
              {m.attached_to()}
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
