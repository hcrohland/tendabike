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
  import { PartNote } from "../lib/partnote";
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";
  import * as m from "../../paraglide/messages";

  let note = $state(new PartNote({}));
  let open = $state(false);

  export function start(n: PartNote) {
    note = n;
    open = true;
  }

  async function onaction() {
    await note.delete();
    open = false;
  }
</script>

<Modal bind:open {onaction}>
  {#snippet header()}
    {m.gearcard_delete_confirm({ name: note.name })}
  {/snippet}
  {#snippet footer()}
    <Buttons bind:open label={m.action_delete()} />
  {/snippet}
</Modal>
