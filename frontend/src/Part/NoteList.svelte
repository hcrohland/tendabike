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
  import { DropdownItem } from "flowbite-svelte";
  import { PaperClipOutline } from "flowbite-svelte-icons";
  import { Part } from "../lib/part";
  import {
    PartNote,
    partNotes,
    notes_for_part,
    fmtSize,
  } from "../lib/partnote";
  import Menu from "../Widgets/Menu.svelte";
  import { actions } from "../Widgets/Actions.svelte";
  import * as m from "../../paraglide/messages";

  interface Props {
    part: Part;
  }

  let { part }: Props = $props();

  let notes = $derived(notes_for_part($partNotes, part.id!));

  function fileTooltip(note: PartNote) {
    const size = fmtSize(note.size ?? 0);
    return note.filename ? `${note.filename} — ${size}` : size;
  }
</script>

<div class="flex flex-col gap-3">
  {#each notes as note (note.id)}
    <div
      class="flex items-start gap-2 border border-gray-300 dark:border-gray-600 rounded px-3 py-2 mb-1"
    >
      <div
        class="flex-1 min-w-0 text-gray-700 dark:text-gray-300 whitespace-pre-wrap"
      >
        {note.name}
      </div>
      {#if note.hasFile()}
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
        <DropdownItem onclick={() => $actions.newNote(part, note)}>
          {m.gearcard_change_note()}
        </DropdownItem>
        <DropdownItem onclick={() => $actions.deleteNote(note)}>
          {m.gearcard_delete_note()}
        </DropdownItem>
      </Menu>
    </div>
  {/each}
</div>
