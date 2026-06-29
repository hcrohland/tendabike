<script lang="ts">
  import { filterValues, by } from "../lib/mapable";
  import { types } from "../lib/types";
  import UsageChips from "../Usage/UsageChips.svelte";
  import PartLink from "./PartLink.svelte";
  import { parts } from "../lib/part";
  import { attachments } from "../lib/attachment";
  import { DropdownItem } from "flowbite-svelte";
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
  <div
    class="text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400 p-1"
  >
    Attached to
  </div>
  <div class="flex flex-col gap-2">
    {#each atts as att (att.attached)}
      <div
        class="rounded-lg border border-gray-200 dark:border-gray-600 bg-surface-1 p-3"
      >
        <div class="flex items-center justify-between gap-2">
          <div class="flex items-center gap-2 min-w-0">
            {#if $parts[att.gear]}
              <span class="font-medium text-sm">
                <PartLink part={$parts[att.gear]} />
              </span>
              <span class="text-xs text-gray-500 dark:text-gray-400 shrink-0">
                {types[att.hook].prefix}
              </span>
              <span class="text-xs text-gray-500 dark:text-gray-400 shrink-0">
                · {att.fmtTime()}
              </span>
            {:else}
              <span class="text-sm text-gray-500 dark:text-gray-400">N/A</span>
            {/if}
          </div>
          {#if $parts[att.gear]}
            <div class="shrink-0">
              <Menu>
                <DropdownItem onclick={() => $actions.deleteAttachment(att)}>
                  Remove
                </DropdownItem>
              </Menu>
            </div>
          {/if}
        </div>
        <UsageChips id={att.usage} ref={att.idx} light />
      </div>
    {/each}
  </div>
{/if}
