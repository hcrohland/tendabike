<script lang="ts">
  import { DropdownItem } from "flowbite-svelte";
  import Menu from "../Widgets/Menu.svelte";
  import ShowMore from "../Widgets/ShowMore.svelte";
  import UsageChips from "../Usage/UsageChips.svelte";
  import type { Attachment } from "../lib/attachment";
  import { parts } from "../lib/part";
  import { Type } from "../lib/types";
  import { usages } from "../lib/usage";
  import PartLink from "./PartLink.svelte";
  import { actions } from "../Widgets/Actions.svelte";
  import XsButton from "../Widgets/XsButton.svelte";

  export let attachments: Attachment[] = [];
  export let level: number = 0;
  export let prefix = "";
  export let type: Type;

  let show_more = false;
</script>

{#each attachments.map( (att) => ({ att, part: $parts[att.part_id] }), ) as { att, part }, i (att.idx)}
  {#if i == 0}
    <div
      class="rounded-lg border border-gray-200 dark:border-gray-600 bg-gray-50 dark:bg-gray-700 p-3"
      style="margin-left: {level * 1.25}rem"
    >
      <!-- Header row: type · name · menu -->
      <div class="flex items-center justify-between gap-2">
        <div class="flex items-center gap-2 min-w-0">
          {#if attachments.length > 1 || (part && $usages[part.usage].count != $usages[att.usage].count)}
            <ShowMore bind:show_more title="history" />
          {/if}
          <span
            class="text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400 shrink-0"
          >
            {prefix + " " + type.name}
          </span>
          {#if att.isAttached()}
            <span class="font-medium text-sm truncate">
              {#if part}
                <PartLink {part} />
              {:else}
                {att.name}
              {/if}
            </span>
          {/if}
        </div>
        {#if att.isAttached()}
          <div class="shrink-0">
            <Menu>
              <DropdownItem onclick={() => $actions.newService(part)}
                >Log Service</DropdownItem
              >
              <DropdownItem onclick={() => $actions.attachPart(part)}
                >Move part</DropdownItem
              >
              <DropdownItem onclick={() => $actions.replacePart(att)}
                >New {type.name}</DropdownItem
              >
            </Menu>
          </div>
        {:else}
          <XsButton onclick={() => $actions.replacePart(att)}>add</XsButton>
        {/if}
      </div>

      <!-- Attached date -->
      {#if att.isAttached()}
        <div class="text-xs text-gray-500 dark:text-gray-400 mt-1">
          since {att.fmtTime()}
        </div>

        <!-- Current stats -->
        <UsageChips id={part.usage} ref={part.id} />
      {/if}

      <!-- History cards -->
      {#if show_more}
        <div class="mt-3 flex flex-col gap-2">
          {#each attachments.map( (a) => ({ a, p: $parts[a.part_id] }), ) as { a, p }}
            <div
              class="rounded-lg border border-gray-200 dark:border-gray-500 bg-gray-100 dark:bg-gray-600 p-3"
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
                  <span
                    class="text-xs text-gray-500 dark:text-gray-400 shrink-0"
                    >{a.fmtTime()}</span
                  >
                </div>
                {#if p && p.disposed_at == undefined}
                  <div class="shrink-0">
                    <Menu>
                      <DropdownItem onclick={() => $actions.newService(p)}
                        >Log Service</DropdownItem
                      >
                      <DropdownItem onclick={() => $actions.attachPart(p)}
                        >Attach part</DropdownItem
                      >
                      <DropdownItem onclick={() => $actions.replacePart(a)}
                        >Duplicate part</DropdownItem
                      >
                    </Menu>
                  </div>
                {/if}
              </div>
              <UsageChips id={a.usage} ref={a.idx} />
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
{/each}
