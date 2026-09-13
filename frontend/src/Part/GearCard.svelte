<script lang="ts">
  import { Card, Badge } from "flowbite-svelte";
  import { PaperClipOutline } from "flowbite-svelte-icons";
  import { link, push } from "svelte-spa-router";
  import { Part } from "../lib/part";
  import {
    PartNote,
    partNotes,
    notes_for_part,
    fmtSize,
  } from "../lib/partnote";
  import { fmtDate } from "../lib/store";
  import { types } from "../lib/types";
  import { user, users } from "../lib/user";
  import UsageChips from "../Usage/UsageChips.svelte";
  import ServiceBadge from "../Widgets/ServiceBadge.svelte";
  import Menu from "../Widgets/Menu.svelte";
  import XsButton from "../Widgets/XsButton.svelte";
  import { DropdownItem } from "flowbite-svelte";
  import NewNote from "./NewNote.svelte";
  import DeleteNote from "./DeleteNote.svelte";
  import * as m from "../../paraglide/messages";

  interface Props {
    part: Part;
    summary?: boolean;
    dues?: any;
    gridclass?: string;
    children?: import("svelte").Snippet;
  }

  let { part, summary = false, dues, gridclass, children }: Props = $props();

  let notes = $derived(notes_for_part($partNotes, part.id!));

  let newNote = $state<{ start: (partId: number, note?: PartNote) => void }>();
  let deleteNote = $state<{ start: (note: PartNote) => void }>();

  function model(part: Part) {
    if (part.model == "" && part.vendor == "") {
      return m.gearcard_unknown_model();
    } else {
      return part.vendor + " " + part.model;
    }
  }

  function typeName(part: Part) {
    if (part.what != types[part.what].main) {
      return types[part.what].localizedName();
    }
    return "";
  }

  function fileTooltip(note: PartNote) {
    const size = fmtSize(note.size ?? 0);
    return note.filename ? `${note.filename} — ${size}` : size;
  }
</script>

<Card size="xl" class="col-auto relative">
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class={"text-xl bg-surface-2 p-2 sm:p-4" +
      (summary
        ? " hover:bg-gray-300 dark:hover:bg-gray-500 cursor-pointer"
        : "")}
    onclick={() => summary && push("/part/" + part.id)}
  >
    {#if part.owner == $user!.id}
      {#if summary}
        <a href="/part/{part.id}" use:link class="text-decoration-none">
          {part.name}
        </a>
      {:else}
        {part.name}
      {/if}
    {:else}
      {@const owner = $users[part.owner]}
      {part.name}
      <Badge color="green" class="relative -top-3 -right-1">
        {owner.firstname + " " + owner.name}
      </Badge>
    {/if}
    {@render children?.()}
  </div>

  <div class="p-2 sm:p-4 bg-surface-1">
    <!-- Meta line: model · type · date range -->
    <p class="text-sm mb-3">
      {model(part)}{typeName(part) ? " · " + typeName(part) : ""}
      {#if part.what == 1}
        <a href={"/strava/bikes/" + part.id} target="_blank">
          <img
            src="strava_grey.png"
            alt={m.gearcard_view_on_strava()}
            title={m.gearcard_view_on_strava()}
            class="inline ml-1"
          />
        </a>
      {/if}
      <span class="text-text-1">
        ·
        {#if !part.disposed_at}
          {m.time_since()} {fmtDate(part.purchase)}
        {:else}
          {fmtDate(part.purchase)} – {fmtDate(part.disposed_at)}
        {/if}
        <ServiceBadge service={dues?.days} />
      </span>
    </p>

    <!-- Stat chips -->
    <UsageChips id={part.usage} ref={part.id} {gridclass} {dues} light />

    <!-- Notes (detail view only) -->
    {#if !summary}
      <div class="mt-4">
        <div class="flex items-center mb-2 my-2">
          <strong>{m.gearcard_notes()}:</strong>
          <XsButton onclick={() => newNote!.start(part.id!)}
            >{m.partcard_add()}</XsButton
          >
        </div>

        {#each notes as note (note.id)}
          <div
            class="flex items-start gap-2 border border-gray-300 dark:border-gray-600 rounded px-3 py-2 mb-1"
          >
            <div
              class="flex-1 min-w-0 text-gray-700 dark:text-gray-300 whitespace-pre-wrap"
            >
              {note.name}
            </div>
            {#if note.kind === "file"}
              <span
                class="flex items-center gap-1 text-gray-500 dark:text-gray-400 text-sm shrink-0"
              >
                <a
                  href={note.fileUrl()}
                  target="_blank"
                  title={fileTooltip(note)}
                  class="hover:text-gray-700 dark:hover:text-gray-300"
                >
                  <PaperClipOutline class="w-4 h-4" />
                </a>
              </span>
            {/if}
            <Menu>
              <DropdownItem onclick={() => newNote!.start(part.id!, note)}>
                {m.gearcard_change_note()}
              </DropdownItem>
              <DropdownItem onclick={() => deleteNote!.start(note)}>
                {m.gearcard_delete_note()}
              </DropdownItem>
            </Menu>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</Card>

{#if !summary}
  <NewNote bind:this={newNote} />
  <DeleteNote bind:this={deleteNote} />
{/if}
