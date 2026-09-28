<!--
	tendabike - the bike maintenance tracker
  	
	Copyright (C) 2023-2026 Christoph Rohland

	This program is free software: you can redistribute it and/or modify
	it under the terms of the GNU Affero General Public License as published
	by the Free Software Foundation, either version 3 of the License, or
	(at your option) any later version.

	This program is distributed in the hope that it will be useful,
	but WITHOUT ANY WARRANTY; without even the implied warranty of
	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
	GNU Affero General Public License for more details.

	You should have received a copy of the GNU Affero General Public License
	along with this program.  If not, see <https://www.gnu.org/licenses/>.
-->
<script module lang="ts">
  import { PartNote, fmtSize } from "../lib/partnote";
  import type { Part } from "../lib/part";

  const modal = $state<{
    open: boolean;
    partId: number;
    name: string;
    file: File | null;
    filePreview: string | null;
    removeFile: boolean;
    editingNote: PartNote | null;
  }>({
    open: false,
    partId: 0,
    name: "",
    file: null,
    filePreview: null,
    removeFile: false,
    editingNote: null,
  });

  function setFile(f: File | null) {
    if (modal.filePreview) URL.revokeObjectURL(modal.filePreview);
    modal.file = f;
    modal.filePreview = f ? URL.createObjectURL(f) : null;
    if (f) modal.removeFile = false;
  }

  export function start(part: Part, note?: PartNote) {
    modal.partId = part.id!;
    if (note) {
      modal.editingNote = note;
      modal.name = note.name;
    } else {
      modal.editingNote = null;
      modal.name = "";
    }
    setFile(null);
    modal.removeFile = false;
    modal.open = true;
  }
</script>

<script lang="ts">
  import { Button, Textarea } from "flowbite-svelte";
  import {
    EditOutline,
    PaperClipOutline,
    TrashBinOutline,
  } from "flowbite-svelte-icons";
  import { onDestroy } from "svelte";
  import { createTextNote, createFileNote } from "../lib/partnote";
  import { handleError } from "../lib/store";
  import Modal from "../Widgets/Modal.svelte";
  import * as m from "../../paraglide/messages";

  let fileInput = $state<HTMLInputElement | null>(null);

  onDestroy(() => {
    if (modal.filePreview) URL.revokeObjectURL(modal.filePreview);
  });

  function pickFile() {
    modal.removeFile = false;
    fileInput?.click();
  }

  function onRemoveFile() {
    modal.removeFile = true;
    setFile(null);
  }

  async function onaction() {
    try {
      if (modal.editingNote) {
        if (modal.removeFile) {
          await modal.editingNote.removeFile();
        } else if (modal.file) {
          await modal.editingNote.updateFile(modal.name.trim(), modal.file);
        } else {
          await modal.editingNote.updateName(modal.name.trim());
        }
      } else {
        if (modal.file) {
          await createFileNote(modal.partId, modal.name.trim(), modal.file);
        } else {
          await createTextNote(modal.partId, modal.name.trim());
        }
      }
    } catch (e: any) {
      handleError(e);
      return;
    }
    setFile(null);
    modal.open = false;
  }
</script>

<Modal bind:open={modal.open} {onaction}>
  {#snippet header()}
    {#if modal.editingNote}
      {m.gearcard_change_note()}
    {:else}
      {m.gearcard_new_note()}
    {/if}
  {/snippet}

  <div class="flex flex-col gap-4">
    <div>
      <label class="block text-sm font-medium mb-1" for="note-name">
        {m.gearcard_note_name()}
      </label>
      <Textarea
        id="note-name"
        bind:value={modal.name}
        rows={3}
        class="w-full"
      />
    </div>

    {#if modal.editingNote?.hasFile()}
      <div
        class="flex items-center gap-3 p-3 bg-gray-50 dark:bg-gray-800 rounded {modal.removeFile
          ? 'opacity-50'
          : ''}"
      >
        {#if modal.filePreview && modal.file?.type.startsWith("image/")}
          <img
            src={modal.filePreview}
            alt={modal.file?.name ?? ""}
            class="max-h-16 rounded"
          />
        {:else if modal.editingNote.hasImage()}
          <img
            src={modal.editingNote.fileUrl()}
            alt={modal.editingNote.filename ?? modal.editingNote.name}
            class="max-h-16 rounded"
          />
        {:else}
          <PaperClipOutline class="w-5 h-5 text-gray-500" />
        {/if}
        <div class="text-sm text-gray-600 dark:text-gray-400 flex-1">
          {modal.file ? modal.file.name : modal.editingNote.filename}
          <span class="ml-2 text-gray-400">
            {fmtSize(
              modal.file ? modal.file.size : (modal.editingNote.size ?? 0),
            )}
          </span>
        </div>
        <button
          type="button"
          onclick={pickFile}
          class="p-1 rounded hover:bg-gray-200 dark:hover:bg-gray-700"
          title={m.newnote_change_file()}
        >
          <EditOutline class="w-4 h-4 text-gray-500" />
        </button>
        <button
          type="button"
          onclick={onRemoveFile}
          class="p-1 rounded hover:bg-red-100 dark:hover:bg-red-900"
          title={m.newnote_remove_file()}
        >
          <TrashBinOutline class="w-4 h-4 text-red-500" />
        </button>
      </div>
      <input
        type="file"
        class="hidden"
        bind:this={fileInput}
        onchange={(e) =>
          setFile((e.target as HTMLInputElement).files?.[0] ?? null)}
      />
    {:else}
      <div>
        <label class="block text-sm font-medium mb-1" for="note-file">
          {m.gearcard_note_file()}
        </label>
        <input
          id="note-file"
          type="file"
          onchange={(e) =>
            setFile((e.target as HTMLInputElement).files?.[0] ?? null)}
        />
      </div>
    {/if}
  </div>

  {#snippet footer()}
    <div class="flex justify-end w-full gap-2">
      <Button color="alternative" onclick={() => (modal.open = false)}>
        {m.action_cancel()}
      </Button>
      <Button
        type="submit"
        value="commit"
        color="gray"
        disabled={!modal.removeFile &&
          !(modal.name.trim().length > 0 || modal.file !== null)}
      >
        {#if modal.editingNote}
          {m.action_update()}
        {:else}
          {m.action_create()}
        {/if}
      </Button>
    </div>
  {/snippet}
</Modal>
