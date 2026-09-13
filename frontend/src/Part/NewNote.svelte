<!-- 
	tendabike - the bike maintenance tracker
  	
	Copyright (C) 2023  Christoph Rohland 

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
<script lang="ts">
  import { Button, Textarea } from "flowbite-svelte";
  import {
    EditOutline,
    PaperClipOutline,
    TrashBinOutline,
  } from "flowbite-svelte-icons";
  import { onDestroy } from "svelte";
  import {
    PartNote,
    createTextNote,
    createFileNote,
    fmtSize,
  } from "../lib/partnote";
  import { handleError } from "../lib/store";
  import Modal from "../Widgets/Modal.svelte";
  import * as m from "../../paraglide/messages";

  let open = $state(false);
  let partId = $state(0);
  let name = $state("");
  let file = $state<File | null>(null);
  let filePreview = $state<string | null>(null);
  let removeFile = $state(false);
  let editingNote: PartNote | null = $state(null);
  let fileInput = $state<HTMLInputElement | null>(null);

  function setFile(f: File | null) {
    if (filePreview) URL.revokeObjectURL(filePreview);
    file = f;
    filePreview = f ? URL.createObjectURL(f) : null;
    if (f) removeFile = false;
  }

  onDestroy(() => {
    if (filePreview) URL.revokeObjectURL(filePreview);
  });

  export function start(pid: number, note?: PartNote) {
    partId = pid;
    if (note) {
      editingNote = note;
      name = note.name;
      setFile(null);
    } else {
      editingNote = null;
      name = "";
      setFile(null);
    }
    removeFile = false;
    open = true;
  }

  function pickFile() {
    removeFile = false;
    fileInput?.click();
  }

  function onRemoveFile() {
    removeFile = true;
    setFile(null);
  }

  async function onaction() {
    try {
      if (editingNote) {
        if (removeFile) {
          await editingNote.removeFile();
        } else if (file) {
          await editingNote.updateFile(name.trim(), file);
        } else {
          await editingNote.updateName(name.trim());
        }
      } else {
        if (file) {
          await createFileNote(partId, name.trim(), file);
        } else {
          await createTextNote(partId, name.trim());
        }
      }
    } catch (e: any) {
      handleError(e);
      return;
    }
    setFile(null);
    open = false;
  }
</script>

<Modal bind:open {onaction} valid={name.trim().length > 0}>
  {#snippet header()}
    {#if editingNote}
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
      <Textarea id="note-name" bind:value={name} rows={3} class="w-full" />
    </div>

    {#if editingNote?.kind === "file"}
      <div
        class="flex items-center gap-3 p-3 bg-gray-50 dark:bg-gray-800 rounded {removeFile
          ? 'opacity-50'
          : ''}"
      >
        {#if filePreview && file?.type.startsWith("image/")}
          <img
            src={filePreview}
            alt={file?.name ?? ""}
            class="max-h-16 rounded"
          />
        {:else if editingNote.isImage()}
          <img
            src={editingNote.fileUrl()}
            alt={editingNote.filename ?? editingNote.name}
            class="max-h-16 rounded"
          />
        {:else}
          <PaperClipOutline class="w-5 h-5 text-gray-500" />
        {/if}
        <div class="text-sm text-gray-600 dark:text-gray-400 flex-1">
          {file ? file.name : editingNote.filename}
          <span class="ml-2 text-gray-400">
            {fmtSize(file ? file.size : (editingNote.size ?? 0))}
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
      <Button color="alternative" onclick={() => (open = false)}>
        {m.action_cancel()}
      </Button>
      <Button type="submit" value="commit" color="gray">
        {#if editingNote}
          {m.action_update()}
        {:else}
          {m.action_create()}
        {/if}
      </Button>
    </div>
  {/snippet}
</Modal>
