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
  import { PartNote } from "../lib/partnote";

  const modal = $state<{
    open: boolean;
    note: PartNote;
  }>({
    open: false,
    note: new PartNote({}),
  });

  export function deleteNote(n: PartNote) {
    modal.note = n;
    modal.open = true;
  }
</script>

<script lang="ts">
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";
  import * as m from "../../paraglide/messages";

  async function onaction() {
    await modal.note.delete();
    modal.open = false;
  }
</script>

<Modal bind:open={modal.open} {onaction}>
  {#snippet header()}
    {m.gearcard_delete_confirm({ name: modal.note.name })}
  {/snippet}
  {#snippet footer()}
    <Buttons bind:open={modal.open} label={m.action_delete()} />
  {/snippet}
</Modal>
