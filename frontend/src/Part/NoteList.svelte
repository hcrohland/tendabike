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

<script lang="ts">
  import { Part } from "../lib/part";
  import { partNotes, notes_for_part } from "../lib/partnote";
  import NoteCard from "./NoteCard.svelte";

  interface Props {
    part: Part;
  }

  let { part }: Props = $props();

  let notes = $derived(notes_for_part($partNotes, part.id!));
</script>

<div class="flex flex-col gap-3">
  {#each notes as note (note.id)}
    <NoteCard {part} {note} />
  {/each}
</div>
