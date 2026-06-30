<script lang="ts">
  import { DropdownItem } from "flowbite-svelte";
  import Menu from "../Widgets/Menu.svelte";
  import ShowMore from "../Widgets/ShowMore.svelte";
  import UsageChips from "../Usage/UsageChips.svelte";
  import type { Attachment } from "../lib/attachment";
  import { attachments as atts } from "../lib/attachment";
  import { parts } from "../lib/part";
  import { Type } from "../lib/types";
  import { usages } from "../lib/usage";
  import { plans, plans_for_part, next_due } from "../lib/serviceplan";
  import PartLink from "./PartLink.svelte";
  import { actions } from "../Widgets/Actions.svelte";
  import XsButton from "../Widgets/XsButton.svelte";
  import { services } from "../lib/service";
  import ServiceBadge from "../Widgets/ServiceBadge.svelte";

  type TreeNode = {
    attachments: Attachment[];
    prefix: string;
    type: Type;
    children: TreeNode[];
  };

  export let attachments: Attachment[] = [];
  export let prefix = "";
  export let type: Type;
  export let children: TreeNode[] = [];
  export let light = false;

  let background = light ? "bg-surface-1" : "bg-surface-2";
  let background2 = !light ? "bg-surface-1" : "bg-surface-2";
  let show_more = false;

  $: list = attachments.map((att) => ({ att, part: $parts[att.part_id] }));
  $: att = list[0]?.att;
  $: part = list[0]?.part;
  $: dues = next_due(
    part,
    plans_for_part($plans, $atts, part?.id),
    $services,
    $usages,
  );
</script>

{#if att}
  <div
    class={"relative rounded-lg border border-border-subtle p-1 md:p-3 " +
      background}
  >
    <!-- Header row: type · name · menu -->
    <div class="flex items-center justify-between gap-2">
      <div class="flex items-center gap-2 p-2 min-w-0">
        <span class="text-xs uppercase text-text-1 shrink-0 truncate">
          {prefix + " " + type.name}
        </span>
        {#if att.isAttached()}
          <span class="font-medium text-sm whitespace-nowrap">
            {#if part}
              <PartLink {part} />
            {:else}
              {att?.name}
            {/if}
          </span>
          ·
          <span class="text-xs text-text-1">
            since {att.fmtTime()}
          </span>
          <ServiceBadge service={dues?.days} />
          {#if attachments.length > 1 || (part && $usages[part.usage].count != $usages[att.usage].count)}
            <ShowMore bind:show_more title="history" />
          {/if}
        {/if}
      </div>
      <div class="flex items-center gap-2 shrink-0">
        {#if att.isAttached() && part}
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
        {:else}
          <XsButton onclick={() => $actions.replacePart(att)}>add</XsButton>
        {/if}
      </div>
    </div>

    <!-- Current stats -->
    {#if att.isAttached()}
      <UsageChips id={part.usage} ref={part.id} {light} {dues} />
    {/if}
    <!-- History cards -->
    {#if show_more}
      <div class="mt-3 flex flex-col gap-2 p-3">
        {#each list as { att: a, part: p } (a.idx)}
          <div
            class={"rounded-lg border border-border-strong opacity-70 p-3 " +
              background2}
          >
            <div class="flex items-center justify-between gap-2">
              <div class="flex items-center gap-2 min-w-0">
                <span class="font-medium text-sm truncate">
                  {#if p}
                    <PartLink part={p} />
                  {:else}
                    {a.name}
                  {/if}
                </span>
                <span class="text-xs text-text-1 shrink-0">
                  {a.fmtTime()}
                </span>
              </div>
              {#if p && p.disposed_at == undefined}
                <div class="shrink-0">
                  <Menu>
                    <DropdownItem onclick={() => $actions.newService(p)}>
                      Log Service
                    </DropdownItem>
                    <DropdownItem onclick={() => $actions.attachPart(p)}>
                      Attach part
                    </DropdownItem>
                    <DropdownItem onclick={() => $actions.replacePart(a)}>
                      Duplicate part
                    </DropdownItem>
                  </Menu>
                </div>
              {/if}
            </div>
            <UsageChips id={a.usage} ref={a.idx} light={!light} />
          </div>
        {/each}
      </div>
    {/if}

    <!-- Child cards nested inside -->
    {#if children.length > 0}
      <div class="mt-3 flex flex-col gap-2">
        {#each children as child (child.type.id)}
          <svelte:self {...child} light={!light} />
        {/each}
      </div>
    {/if}
  </div>
{/if}
