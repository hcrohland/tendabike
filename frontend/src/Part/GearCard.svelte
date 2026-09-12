<script lang="ts">
  import { Card, Textarea, Button, Badge } from "flowbite-svelte";
  import { EditOutline } from "flowbite-svelte-icons";
  import { link, push } from "svelte-spa-router";
  import { Part } from "../lib/part";
  import {
    PartNote,
    partNotes,
    notes_for_part,
    createTextNote,
    createFileNote,
  } from "../lib/partnote";
  import { fmtDate } from "../lib/store";
  import { types } from "../lib/types";
  import { user, users } from "../lib/user";
  import UsageChips from "../Usage/UsageChips.svelte";
  import ServiceBadge from "../Widgets/ServiceBadge.svelte";
  import * as m from "../../paraglide/messages";

  interface Props {
    part: Part;
    summary?: boolean;
    dues?: any;
    gridclass?: string;
    children?: import("svelte").Snippet;
  }

  let { part, summary = false, dues, gridclass, children }: Props = $props();

  let addingNote = $state(false);
  let newText = $state("");
  let fileInput = $state<HTMLInputElement>();

  let editingId = $state<number | null>(null);
  let editValue = $state("");

  let notes = $derived(notes_for_part($partNotes, part.id!));

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

  async function addTextNote() {
    if (!newText.trim()) return;
    await createTextNote(part.id!, newText.trim());
    newText = "";
    addingNote = false;
  }

  async function onFileSelected() {
    const file = fileInput?.files?.[0];
    if (!file) return;
    await createFileNote(part.id!, file);
    if (fileInput) fileInput.value = "";
  }

  async function deleteNote(note: PartNote) {
    await note.delete();
  }

  function startEdit(note: PartNote) {
    editingId = note.id ?? null;
    editValue = note.name;
  }

  async function saveEdit(note: PartNote) {
    await note.updateText(editValue);
    editingId = null;
    editValue = "";
  }

  function fmtSize(bytes: number) {
    if (bytes < 1024) return bytes + " B";
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
    return (bytes / (1024 * 1024)).toFixed(1) + " MB";
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

    <!-- Notes & attachments (detail view only) -->
    {#if !summary}
      <div class="mt-4">
        <div class="flex items-center gap-2 mb-2">
          <strong>{m.gearcard_notes()}:</strong>
        </div>

        <!-- Existing notes -->
        {#each notes as note (note.id)}
          <div class="flex items-start gap-2 mb-2">
            {#if note.kind === "text"}
              <div class="flex-1">
                {#if editingId == note.id}
                  <div class="flex flex-col gap-2">
                    <Textarea bind:value={editValue} rows={2} class="w-full" />
                    <div class="flex gap-2">
                      <Button size="sm" onclick={() => saveEdit(note)}>
                        {m.gearcard_save()}
                      </Button>
                      <Button
                        size="sm"
                        color="alternative"
                        onclick={() => {
                          editingId = null;
                          editValue = "";
                        }}
                      >
                        {m.gearcard_cancel()}
                      </Button>
                    </div>
                  </div>
                {:else}
                  <p
                    class="text-gray-700 dark:text-gray-300 whitespace-pre-wrap"
                  >
                    {note.name}
                  </p>
                {/if}
              </div>
              <div class="flex gap-1">
                <EditOutline
                  class="w-4 h-4 cursor-pointer text-gray-400 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200"
                  onclick={() => startEdit(note)}
                />
                <button
                  class="text-red-500 hover:text-red-700 dark:hover:text-red-300 text-xs"
                  onclick={() => deleteNote(note)}
                  title={m.gearcard_delete()}
                >
                  ✕
                </button>
              </div>
            {:else}
              <div class="flex-1">
                {#if note.isImage()}
                  <a href={note.fileUrl()} target="_blank">
                    <img
                      src={note.fileUrl()}
                      alt={note.name}
                      class="max-h-32 rounded"
                    />
                  </a>
                {:else}
                  <a
                    href={note.fileUrl()}
                    class="text-blue-600 dark:text-blue-400 hover:underline"
                  >
                    {note.name}
                  </a>
                {/if}
                <span class="text-xs text-gray-500 ml-2">
                  {fmtSize(note.size ?? 0)}
                </span>
              </div>
              <button
                class="text-red-500 hover:text-red-700 dark:hover:text-red-300 text-xs"
                onclick={() => deleteNote(note)}
                title={m.gearcard_delete()}
              >
                ✕
              </button>
            {/if}
          </div>
        {/each}

        <!-- Add text note -->
        {#if addingNote}
          <div class="mt-2">
            <Textarea
              bind:value={newText}
              placeholder={m.gearcard_notes_placeholder()}
              rows={2}
              class="w-full mb-2"
            />
            <div class="flex gap-2">
              <Button size="sm" onclick={addTextNote}>
                {m.gearcard_add_note()}
              </Button>
              <Button
                size="sm"
                color="alternative"
                onclick={() => (addingNote = false)}
              >
                {m.gearcard_cancel()}
              </Button>
            </div>
          </div>
        {:else}
          <div class="flex gap-2 mt-2">
            <Button
              size="sm"
              color="alternative"
              onclick={() => (addingNote = true)}
            >
              {m.gearcard_add_note()}
            </Button>
            <input
              type="file"
              class="hidden"
              bind:this={fileInput}
              onchange={onFileSelected}
            />
            <Button
              size="sm"
              color="alternative"
              onclick={() => fileInput?.click()}
            >
              {m.gearcard_upload_file()}
            </Button>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</Card>
